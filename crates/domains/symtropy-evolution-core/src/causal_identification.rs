use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, AnalysisContentDigest,
    CausalSelectionIdentificationId, ExplicitSelectionAnalysisFrameDigest,
    IdentificationEvidenceAuthorityId, IdentificationQualificationAuthorityId,
    PredictorRepresentation, SelectionComparisonDesignDigest, SelectionDesignClass,
    SelectionEstimand, ValidatedSelectionAnalysisFrame,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const CAUSAL_SELECTION_IDENTIFICATION_VERSION: u32 = 1;

const IDENTIFICATION_DOMAIN: &[u8] =
    b"symtropy:evolution:causal-selection-identification:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CausalSelectionTarget {
    BinaryViabilityRiskDifference,
}

impl CausalSelectionTarget {
    fn tag(self) -> u8 {
        match self {
            Self::BinaryViabilityRiskDifference => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CausalIdentificationTier {
    InterventionIdentified,
}

impl CausalIdentificationTier {
    fn tag(self) -> u8 {
        match self {
            Self::InterventionIdentified => 0,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum CausalIdentificationCriterion {
    AssignmentMechanism,
    AllocationIntegrity,
    TemporalPrecedence,
    InterventionFidelity,
    InterferencePolicy,
    OutcomeAscertainment,
    AttritionMissingness,
    AnalysisPopulationIntegrity,
    ProtocolFreezeProvenance,
    SimulationReplayIdentity,
    InterventionIsolation,
    ScenarioContext,
    SimulationStochasticityPolicy,
}

impl CausalIdentificationCriterion {
    fn tag(self) -> u8 {
        match self {
            Self::AssignmentMechanism => 0,
            Self::AllocationIntegrity => 1,
            Self::TemporalPrecedence => 2,
            Self::InterventionFidelity => 3,
            Self::InterferencePolicy => 4,
            Self::OutcomeAscertainment => 5,
            Self::AttritionMissingness => 6,
            Self::AnalysisPopulationIntegrity => 7,
            Self::ProtocolFreezeProvenance => 8,
            Self::SimulationReplayIdentity => 9,
            Self::InterventionIsolation => 10,
            Self::ScenarioContext => 11,
            Self::SimulationStochasticityPolicy => 12,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentificationEvidenceQualificationRef {
    pub authority_id: IdentificationQualificationAuthorityId,
    pub revision: u64,
    pub content_digest: AnalysisContentDigest,
}

impl IdentificationEvidenceQualificationRef {
    pub fn new(
        authority_id: IdentificationQualificationAuthorityId,
        revision: u64,
        content_digest: AnalysisContentDigest,
    ) -> Self {
        Self {
            authority_id,
            revision,
            content_digest,
        }
    }

    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.authority_id.as_str());
        put_u64(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentificationEvidenceRef {
    pub criterion: CausalIdentificationCriterion,
    pub authority_id: IdentificationEvidenceAuthorityId,
    pub revision: u64,
    pub content_digest: AnalysisContentDigest,
    pub qualification: IdentificationEvidenceQualificationRef,
    pub subject_design_digest: SelectionComparisonDesignDigest,
    pub subject_frame_digest: ExplicitSelectionAnalysisFrameDigest,
}

impl IdentificationEvidenceRef {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        criterion: CausalIdentificationCriterion,
        authority_id: IdentificationEvidenceAuthorityId,
        revision: u64,
        content_digest: AnalysisContentDigest,
        qualification: IdentificationEvidenceQualificationRef,
        subject_design_digest: SelectionComparisonDesignDigest,
        subject_frame_digest: ExplicitSelectionAnalysisFrameDigest,
    ) -> Self {
        Self {
            criterion,
            authority_id,
            revision,
            content_digest,
            qualification,
            subject_design_digest,
            subject_frame_digest,
        }
    }

    fn validate_subject(
        &self,
        design_digest: SelectionComparisonDesignDigest,
        frame_digest: ExplicitSelectionAnalysisFrameDigest,
    ) -> Result<(), CausalIdentificationError> {
        if self.subject_design_digest != design_digest || self.subject_frame_digest != frame_digest {
            return Err(CausalIdentificationError::CriterionSubjectMismatch(
                self.criterion,
            ));
        }
        Ok(())
    }

    fn update_digest(&self, digest: &mut Sha256) {
        digest.update([self.criterion.tag()]);
        put_text(digest, self.authority_id.as_str());
        put_u64(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
        self.qualification.update_digest(digest);
        digest.update(self.subject_design_digest.as_bytes());
        digest.update(self.subject_frame_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalSelectionIdentification {
    identification_version: u32,
    pub identification_id: CausalSelectionIdentificationId,
    pub tier: CausalIdentificationTier,
    pub target: CausalSelectionTarget,
    pub source_design_class: SelectionDesignClass,
    design_digest: SelectionComparisonDesignDigest,
    frame_digest: ExplicitSelectionAnalysisFrameDigest,
    original_denominator: u64,
    estimand_denominator: u64,
    pub criteria: Vec<IdentificationEvidenceRef>,
}

impl CausalSelectionIdentification {
    pub fn declare(
        identification_id: CausalSelectionIdentificationId,
        frame: &ValidatedSelectionAnalysisFrame<'_>,
        target: CausalSelectionTarget,
        evidence: impl IntoIterator<Item = IdentificationEvidenceRef>,
    ) -> Result<Self, CausalIdentificationError> {
        validate_frame_target(frame, target)?;
        let required = required_criteria(frame.design_class())?;
        let design_digest = frame.design_digest();
        let frame_digest = frame.frame_digest();

        let mut by_criterion = BTreeMap::new();
        for item in evidence {
            item.validate_subject(design_digest, frame_digest)?;
            let criterion = item.criterion;
            if by_criterion.insert(criterion, item).is_some() {
                return Err(CausalIdentificationError::DuplicateCriterion(criterion));
            }
        }

        for criterion in required {
            if !by_criterion.contains_key(criterion) {
                return Err(CausalIdentificationError::MissingCriterion(*criterion));
            }
        }
        if by_criterion.len() != required.len() {
            let extra = by_criterion
                .keys()
                .copied()
                .find(|criterion| !required.contains(criterion))
                .expect("length mismatch must contain an extra criterion");
            return Err(CausalIdentificationError::UnexpectedCriterion(extra));
        }

        let identification = Self {
            identification_version: CAUSAL_SELECTION_IDENTIFICATION_VERSION,
            identification_id,
            tier: CausalIdentificationTier::InterventionIdentified,
            target,
            source_design_class: frame.design_class(),
            design_digest,
            frame_digest,
            original_denominator: frame.frame().original_denominator(),
            estimand_denominator: frame.frame().estimand_denominator(),
            criteria: by_criterion.into_values().collect(),
        };
        identification.validate_local()?;
        Ok(identification)
    }

    pub fn validate_current(
        &self,
        frame: &ValidatedSelectionAnalysisFrame<'_>,
        fresh_evidence: impl IntoIterator<Item = IdentificationEvidenceRef>,
    ) -> Result<(), CausalIdentificationError> {
        self.validate_local()?;
        let recomputed = Self::declare(
            self.identification_id.clone(),
            frame,
            self.target,
            fresh_evidence,
        )?;
        if recomputed != *self {
            return Err(CausalIdentificationError::IdentificationReplayMismatch);
        }
        Ok(())
    }

    pub fn design_digest(&self) -> SelectionComparisonDesignDigest {
        self.design_digest
    }

    pub fn frame_digest(&self) -> ExplicitSelectionAnalysisFrameDigest {
        self.frame_digest
    }

    pub fn original_denominator(&self) -> u64 {
        self.original_denominator
    }

    pub fn estimand_denominator(&self) -> u64 {
        self.estimand_denominator
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<CausalSelectionIdentificationDigest, CausalIdentificationError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(IDENTIFICATION_DOMAIN);
        put_u32(&mut digest, self.identification_version);
        put_text(&mut digest, self.identification_id.as_str());
        digest.update([self.tier.tag(), self.target.tag()]);
        digest.update([design_class_tag(self.source_design_class)]);
        digest.update(self.design_digest.as_bytes());
        digest.update(self.frame_digest.as_bytes());
        put_u64(&mut digest, self.original_denominator);
        put_u64(&mut digest, self.estimand_denominator);
        put_u64(&mut digest, self.criteria.len() as u64);
        for criterion in &self.criteria {
            criterion.update_digest(&mut digest);
        }
        Ok(CausalSelectionIdentificationDigest(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), CausalIdentificationError> {
        if self.identification_version != CAUSAL_SELECTION_IDENTIFICATION_VERSION {
            return Err(CausalIdentificationError::UnsupportedIdentificationVersion(
                self.identification_version,
            ));
        }
        if self.tier != CausalIdentificationTier::InterventionIdentified {
            return Err(CausalIdentificationError::UnsupportedIdentificationTier);
        }
        validate_design_class(self.source_design_class)?;
        if self.target != CausalSelectionTarget::BinaryViabilityRiskDifference {
            return Err(CausalIdentificationError::UnsupportedTarget);
        }
        if self.estimand_denominator == 0
            || self.estimand_denominator > self.original_denominator
        {
            return Err(CausalIdentificationError::DenominatorInvariant);
        }
        let required = required_criteria(self.source_design_class)?;
        if self.criteria.len() != required.len() {
            return Err(CausalIdentificationError::CriterionSetMismatch);
        }
        if self
            .criteria
            .windows(2)
            .any(|window| window[0].criterion >= window[1].criterion)
        {
            return Err(CausalIdentificationError::NonCanonicalCriterionOrder);
        }
        for item in &self.criteria {
            item.validate_subject(self.design_digest, self.frame_digest)?;
            if !required.contains(&item.criterion) {
                return Err(CausalIdentificationError::UnexpectedCriterion(item.criterion));
            }
        }
        for criterion in required {
            if !self.criteria.iter().any(|item| item.criterion == *criterion) {
                return Err(CausalIdentificationError::MissingCriterion(*criterion));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CausalSelectionIdentificationDigest([u8; 32]);

impl CausalSelectionIdentificationDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for CausalSelectionIdentificationDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CausalSelectionIdentificationDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for CausalSelectionIdentificationDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated causal-identification authority should gate causal-effect execution"]
pub struct ValidatedCausalSelectionIdentification<'a> {
    identification: &'a CausalSelectionIdentification,
    identification_digest: CausalSelectionIdentificationDigest,
    design_digest: SelectionComparisonDesignDigest,
    frame_digest: ExplicitSelectionAnalysisFrameDigest,
    tier: CausalIdentificationTier,
    target: CausalSelectionTarget,
}

impl<'a> ValidatedCausalSelectionIdentification<'a> {
    pub fn validate_current(
        identification: &'a CausalSelectionIdentification,
        frame: &ValidatedSelectionAnalysisFrame<'_>,
        fresh_evidence: impl IntoIterator<Item = IdentificationEvidenceRef>,
    ) -> Result<Self, CausalIdentificationError> {
        identification.validate_current(frame, fresh_evidence)?;
        Ok(Self {
            identification,
            identification_digest: identification.canonical_digest()?,
            design_digest: frame.design_digest(),
            frame_digest: frame.frame_digest(),
            tier: identification.tier,
            target: identification.target,
        })
    }

    pub fn identification(&self) -> &'a CausalSelectionIdentification {
        self.identification
    }

    pub fn identification_digest(&self) -> CausalSelectionIdentificationDigest {
        self.identification_digest
    }

    pub fn design_digest(&self) -> SelectionComparisonDesignDigest {
        self.design_digest
    }

    pub fn frame_digest(&self) -> ExplicitSelectionAnalysisFrameDigest {
        self.frame_digest
    }

    pub fn tier(&self) -> CausalIdentificationTier {
        self.tier
    }

    pub fn target(&self) -> CausalSelectionTarget {
        self.target
    }
}

const RANDOMIZED_REQUIRED: &[CausalIdentificationCriterion] = &[
    CausalIdentificationCriterion::AssignmentMechanism,
    CausalIdentificationCriterion::AllocationIntegrity,
    CausalIdentificationCriterion::TemporalPrecedence,
    CausalIdentificationCriterion::InterventionFidelity,
    CausalIdentificationCriterion::InterferencePolicy,
    CausalIdentificationCriterion::OutcomeAscertainment,
    CausalIdentificationCriterion::AttritionMissingness,
    CausalIdentificationCriterion::AnalysisPopulationIntegrity,
    CausalIdentificationCriterion::ProtocolFreezeProvenance,
];

const SIMULATION_REQUIRED: &[CausalIdentificationCriterion] = &[
    CausalIdentificationCriterion::AssignmentMechanism,
    CausalIdentificationCriterion::AllocationIntegrity,
    CausalIdentificationCriterion::TemporalPrecedence,
    CausalIdentificationCriterion::InterventionFidelity,
    CausalIdentificationCriterion::InterferencePolicy,
    CausalIdentificationCriterion::OutcomeAscertainment,
    CausalIdentificationCriterion::AttritionMissingness,
    CausalIdentificationCriterion::AnalysisPopulationIntegrity,
    CausalIdentificationCriterion::ProtocolFreezeProvenance,
    CausalIdentificationCriterion::SimulationReplayIdentity,
    CausalIdentificationCriterion::InterventionIsolation,
    CausalIdentificationCriterion::ScenarioContext,
    CausalIdentificationCriterion::SimulationStochasticityPolicy,
];

fn required_criteria(
    design_class: SelectionDesignClass,
) -> Result<&'static [CausalIdentificationCriterion], CausalIdentificationError> {
    match design_class {
        SelectionDesignClass::RandomizedInterventional => Ok(RANDOMIZED_REQUIRED),
        SelectionDesignClass::ControlledSimulationIntervention => Ok(SIMULATION_REQUIRED),
        _ => Err(CausalIdentificationError::UnsupportedDesignClass(
            design_class,
        )),
    }
}

fn validate_design_class(
    design_class: SelectionDesignClass,
) -> Result<(), CausalIdentificationError> {
    required_criteria(design_class).map(|_| ())
}

fn validate_frame_target(
    frame: &ValidatedSelectionAnalysisFrame<'_>,
    target: CausalSelectionTarget,
) -> Result<(), CausalIdentificationError> {
    validate_design_class(frame.design_class())?;
    match target {
        CausalSelectionTarget::BinaryViabilityRiskDifference => {
            if frame.estimand() != SelectionEstimand::ViabilityWindowRiskContrast {
                return Err(CausalIdentificationError::UnsupportedEstimand(
                    frame.estimand(),
                ));
            }
            if frame.frame().predictor_representation
                != PredictorRepresentation::BinaryComparisonGroup
            {
                return Err(CausalIdentificationError::UnsupportedPredictorRepresentation);
            }
        }
    }
    Ok(())
}

fn design_class_tag(value: SelectionDesignClass) -> u8 {
    match value {
        SelectionDesignClass::DescriptiveAssociation => 0,
        SelectionDesignClass::RandomizedInterventional => 1,
        SelectionDesignClass::ControlledSimulationIntervention => 2,
        SelectionDesignClass::MatchedStratifiedObservational => 3,
        SelectionDesignClass::WithinFamilyOrLineageControlled => 4,
        SelectionDesignClass::CommonEnvironment => 5,
        SelectionDesignClass::ReciprocalContext => 6,
        SelectionDesignClass::DeclaredNeutralNullComparison => 7,
    }
}

#[derive(Debug)]
pub enum CausalIdentificationError {
    UnsupportedIdentificationVersion(u32),
    UnsupportedIdentificationTier,
    UnsupportedDesignClass(SelectionDesignClass),
    UnsupportedTarget,
    UnsupportedEstimand(SelectionEstimand),
    UnsupportedPredictorRepresentation,
    DuplicateCriterion(CausalIdentificationCriterion),
    MissingCriterion(CausalIdentificationCriterion),
    UnexpectedCriterion(CausalIdentificationCriterion),
    CriterionSubjectMismatch(CausalIdentificationCriterion),
    CriterionSetMismatch,
    NonCanonicalCriterionOrder,
    DenominatorInvariant,
    IdentificationReplayMismatch,
}

impl fmt::Display for CausalIdentificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedIdentificationVersion(version) => {
                write!(f, "unsupported causal-identification version {version}")
            }
            Self::UnsupportedIdentificationTier => {
                write!(f, "unsupported causal-identification tier")
            }
            Self::UnsupportedDesignClass(class) => write!(
                f,
                "SEL-07C1 V1 does not permit causal identification for design class {class:?}"
            ),
            Self::UnsupportedTarget => write!(f, "unsupported causal-selection target"),
            Self::UnsupportedEstimand(estimand) => write!(
                f,
                "causal target is incompatible with selection estimand {estimand:?}"
            ),
            Self::UnsupportedPredictorRepresentation => write!(
                f,
                "causal viability target requires binary reference/comparison predictor representation"
            ),
            Self::DuplicateCriterion(criterion) => {
                write!(f, "causal-identification criterion {criterion:?} appears more than once")
            }
            Self::MissingCriterion(criterion) => {
                write!(f, "required causal-identification criterion {criterion:?} is missing")
            }
            Self::UnexpectedCriterion(criterion) => write!(
                f,
                "causal-identification criterion {criterion:?} is not permitted for this design class"
            ),
            Self::CriterionSubjectMismatch(criterion) => write!(
                f,
                "causal-identification evidence for {criterion:?} is bound to another design/frame"
            ),
            Self::CriterionSetMismatch => {
                write!(f, "causal-identification criterion set is incomplete or contains extras")
            }
            Self::NonCanonicalCriterionOrder => {
                write!(f, "causal-identification criteria are not in canonical order")
            }
            Self::DenominatorInvariant => {
                write!(f, "causal-identification denominator invariant failed")
            }
            Self::IdentificationReplayMismatch => {
                write!(f, "causal-identification replay mismatch")
            }
        }
    }
}

impl Error for CausalIdentificationError {}
