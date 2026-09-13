use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, AnalysisContentDigest,
    AssignmentExchangeabilityAuthorityId, AssignmentMaterializationAuthorityId,
    BinaryComparisonGroup, CausalIdentificationCriterion, CausalIdentificationTier,
    CausalSelectionIdentification, CausalSelectionIdentificationDigest, CausalSelectionTarget,
    CompleteRandomizationReferenceId, EvolutionIndividualId, ExplicitSelectionAnalysisFrameDigest,
    IdentificationEvidenceRef, MaterializedPredictorValue, PredictorMaterializationStatus,
    PredictorRepresentation, SelectionComparisonDesignDigest, SelectionDesignClass,
    SelectionEstimand, ValidatedCausalSelectionIdentification, ValidatedSelectionAnalysisFrame,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const COMPLETE_RANDOMIZATION_REFERENCE_VERSION: u32 = 1;
pub const COMPLETE_RANDOMIZATION_MAX_UNITS: u64 = 64;

const REFERENCE_DOMAIN: &[u8] =
    b"symtropy:evolution:complete-randomization-reference:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RandomizationCausalEstimand {
    IntentionToTreatBinaryViabilityRiskDifference,
}

impl RandomizationCausalEstimand {
    fn tag(self) -> u8 {
        match self {
            Self::IntentionToTreatBinaryViabilityRiskDifference => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignmentUnitPolicy {
    PersistentIndividual,
}

impl AssignmentUnitPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::PersistentIndividual => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignmentSupportPolicy {
    CompleteFixedComparisonCount,
}

impl AssignmentSupportPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::CompleteFixedComparisonCount => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignmentMechanismMaterializationRef {
    pub authority_id: AssignmentMaterializationAuthorityId,
    pub revision: u64,
    pub content_digest: AnalysisContentDigest,
    pub subject_identification_digest: CausalSelectionIdentificationDigest,
}

impl AssignmentMechanismMaterializationRef {
    pub fn new(
        authority_id: AssignmentMaterializationAuthorityId,
        revision: u64,
        content_digest: AnalysisContentDigest,
        subject_identification_digest: CausalSelectionIdentificationDigest,
    ) -> Self {
        Self {
            authority_id,
            revision,
            content_digest,
            subject_identification_digest,
        }
    }

    fn validate_subject(
        &self,
        identification_digest: CausalSelectionIdentificationDigest,
    ) -> Result<(), CompleteRandomizationError> {
        if self.subject_identification_digest != identification_digest {
            return Err(CompleteRandomizationError::MaterializationSubjectMismatch);
        }
        Ok(())
    }

    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.authority_id.as_str());
        put_u64(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
        digest.update(self.subject_identification_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndividualAssignmentExchangeabilityRef {
    pub authority_id: AssignmentExchangeabilityAuthorityId,
    pub revision: u64,
    pub content_digest: AnalysisContentDigest,
    pub subject_identification_digest: CausalSelectionIdentificationDigest,
}

impl IndividualAssignmentExchangeabilityRef {
    pub fn new(
        authority_id: AssignmentExchangeabilityAuthorityId,
        revision: u64,
        content_digest: AnalysisContentDigest,
        subject_identification_digest: CausalSelectionIdentificationDigest,
    ) -> Self {
        Self {
            authority_id,
            revision,
            content_digest,
            subject_identification_digest,
        }
    }

    fn validate_subject(
        &self,
        identification_digest: CausalSelectionIdentificationDigest,
    ) -> Result<(), CompleteRandomizationError> {
        if self.subject_identification_digest != identification_digest {
            return Err(CompleteRandomizationError::ExchangeabilitySubjectMismatch);
        }
        Ok(())
    }

    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.authority_id.as_str());
        put_u64(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
        digest.update(self.subject_identification_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealizedAssignmentUnit {
    pub individual_id: EvolutionIndividualId,
    pub group: BinaryComparisonGroup,
}

impl RealizedAssignmentUnit {
    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.individual_id.as_str());
        digest.update([group_tag(self.group)]);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompleteRandomizationReferenceModel {
    reference_version: u32,
    pub reference_id: CompleteRandomizationReferenceId,
    pub estimand: RandomizationCausalEstimand,
    pub assignment_unit_policy: AssignmentUnitPolicy,
    pub support_policy: AssignmentSupportPolicy,
    pub source_design_class: SelectionDesignClass,
    pub causal_target: CausalSelectionTarget,
    identification_digest: CausalSelectionIdentificationDigest,
    design_digest: SelectionComparisonDesignDigest,
    frame_digest: ExplicitSelectionAnalysisFrameDigest,
    original_denominator: u64,
    estimand_denominator: u64,
    pub reference_count: u64,
    pub comparison_count: u64,
    pub support_size: u64,
    pub assignment_mechanism: IdentificationEvidenceRef,
    pub allocation_integrity: IdentificationEvidenceRef,
    pub temporal_precedence: IdentificationEvidenceRef,
    pub intervention_fidelity: IdentificationEvidenceRef,
    pub interference_policy: IdentificationEvidenceRef,
    pub analysis_population_integrity: IdentificationEvidenceRef,
    pub protocol_freeze_provenance: IdentificationEvidenceRef,
    pub mechanism_materialization: AssignmentMechanismMaterializationRef,
    pub individual_exchangeability: IndividualAssignmentExchangeabilityRef,
    pub realized_assignments: Vec<RealizedAssignmentUnit>,
}

impl CompleteRandomizationReferenceModel {
    pub fn capture(
        reference_id: CompleteRandomizationReferenceId,
        frame: &ValidatedSelectionAnalysisFrame<'_>,
        identification: &ValidatedCausalSelectionIdentification<'_>,
        mechanism_materialization: AssignmentMechanismMaterializationRef,
        individual_exchangeability: IndividualAssignmentExchangeabilityRef,
    ) -> Result<Self, CompleteRandomizationError> {
        validate_capability_chain(frame, identification)?;

        let identification_digest = identification.identification_digest();
        mechanism_materialization.validate_subject(identification_digest)?;
        individual_exchangeability.validate_subject(identification_digest)?;

        let current_identification = identification.identification();
        let assignment_mechanism = criterion_record(
            current_identification,
            CausalIdentificationCriterion::AssignmentMechanism,
        )?;
        let allocation_integrity = criterion_record(
            current_identification,
            CausalIdentificationCriterion::AllocationIntegrity,
        )?;
        let temporal_precedence = criterion_record(
            current_identification,
            CausalIdentificationCriterion::TemporalPrecedence,
        )?;
        let intervention_fidelity = criterion_record(
            current_identification,
            CausalIdentificationCriterion::InterventionFidelity,
        )?;
        let interference_policy = criterion_record(
            current_identification,
            CausalIdentificationCriterion::InterferencePolicy,
        )?;
        let analysis_population_integrity = criterion_record(
            current_identification,
            CausalIdentificationCriterion::AnalysisPopulationIntegrity,
        )?;
        let protocol_freeze_provenance = criterion_record(
            current_identification,
            CausalIdentificationCriterion::ProtocolFreezeProvenance,
        )?;

        let mut realized_assignments = Vec::with_capacity(frame.frame().rows.len());
        let mut reference_count = 0u64;
        let mut comparison_count = 0u64;
        for row in &frame.frame().rows {
            let group = match &row.predictor.status {
                PredictorMaterializationStatus::Observed(MaterializedPredictorValue::Binary(group)) => {
                    *group
                }
                _ => {
                    return Err(CompleteRandomizationError::UnsupportedRealizedAssignment(
                        row.individual_id.clone(),
                    ))
                }
            };
            match group {
                BinaryComparisonGroup::Reference => {
                    reference_count = reference_count
                        .checked_add(1)
                        .ok_or(CompleteRandomizationError::ArithmeticOverflow)?;
                }
                BinaryComparisonGroup::Comparison => {
                    comparison_count = comparison_count
                        .checked_add(1)
                        .ok_or(CompleteRandomizationError::ArithmeticOverflow)?;
                }
            }
            realized_assignments.push(RealizedAssignmentUnit {
                individual_id: row.individual_id.clone(),
                group,
            });
        }

        if reference_count == 0 {
            return Err(CompleteRandomizationError::EmptyReferenceGroup);
        }
        if comparison_count == 0 {
            return Err(CompleteRandomizationError::EmptyComparisonGroup);
        }

        let estimand_denominator = frame.frame().estimand_denominator();
        if estimand_denominator > COMPLETE_RANDOMIZATION_MAX_UNITS {
            return Err(CompleteRandomizationError::ReferencePopulationTooLarge {
                units: estimand_denominator,
                max_units: COMPLETE_RANDOMIZATION_MAX_UNITS,
            });
        }
        let support_size = choose_u64(estimand_denominator, comparison_count)?;

        let model = Self {
            reference_version: COMPLETE_RANDOMIZATION_REFERENCE_VERSION,
            reference_id,
            estimand: RandomizationCausalEstimand::IntentionToTreatBinaryViabilityRiskDifference,
            assignment_unit_policy: AssignmentUnitPolicy::PersistentIndividual,
            support_policy: AssignmentSupportPolicy::CompleteFixedComparisonCount,
            source_design_class: frame.design_class(),
            causal_target: identification.target(),
            identification_digest,
            design_digest: frame.design_digest(),
            frame_digest: frame.frame_digest(),
            original_denominator: frame.frame().original_denominator(),
            estimand_denominator,
            reference_count,
            comparison_count,
            support_size,
            assignment_mechanism,
            allocation_integrity,
            temporal_precedence,
            intervention_fidelity,
            interference_policy,
            analysis_population_integrity,
            protocol_freeze_provenance,
            mechanism_materialization,
            individual_exchangeability,
            realized_assignments,
        };
        model.validate_local()?;
        Ok(model)
    }

    pub fn validate_current(
        &self,
        frame: &ValidatedSelectionAnalysisFrame<'_>,
        identification: &ValidatedCausalSelectionIdentification<'_>,
        mechanism_materialization: AssignmentMechanismMaterializationRef,
        individual_exchangeability: IndividualAssignmentExchangeabilityRef,
    ) -> Result<(), CompleteRandomizationError> {
        self.validate_local()?;
        let recomputed = Self::capture(
            self.reference_id.clone(),
            frame,
            identification,
            mechanism_materialization,
            individual_exchangeability,
        )?;
        if recomputed != *self {
            return Err(CompleteRandomizationError::ReferenceReplayMismatch);
        }
        Ok(())
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

    pub fn original_denominator(&self) -> u64 {
        self.original_denominator
    }

    pub fn estimand_denominator(&self) -> u64 {
        self.estimand_denominator
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<CompleteRandomizationReferenceDigest, CompleteRandomizationError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(REFERENCE_DOMAIN);
        put_u32(&mut digest, self.reference_version);
        put_text(&mut digest, self.reference_id.as_str());
        digest.update([
            self.estimand.tag(),
            self.assignment_unit_policy.tag(),
            self.support_policy.tag(),
            design_class_tag(self.source_design_class),
            target_tag(self.causal_target),
        ]);
        digest.update(self.identification_digest.as_bytes());
        digest.update(self.design_digest.as_bytes());
        digest.update(self.frame_digest.as_bytes());
        put_u64(&mut digest, self.original_denominator);
        put_u64(&mut digest, self.estimand_denominator);
        put_u64(&mut digest, self.reference_count);
        put_u64(&mut digest, self.comparison_count);
        put_u64(&mut digest, self.support_size);
        update_criterion_digest(&mut digest, &self.assignment_mechanism);
        update_criterion_digest(&mut digest, &self.allocation_integrity);
        update_criterion_digest(&mut digest, &self.temporal_precedence);
        update_criterion_digest(&mut digest, &self.intervention_fidelity);
        update_criterion_digest(&mut digest, &self.interference_policy);
        update_criterion_digest(&mut digest, &self.analysis_population_integrity);
        update_criterion_digest(&mut digest, &self.protocol_freeze_provenance);
        self.mechanism_materialization.update_digest(&mut digest);
        self.individual_exchangeability.update_digest(&mut digest);
        put_u64(&mut digest, self.realized_assignments.len() as u64);
        for assignment in &self.realized_assignments {
            assignment.update_digest(&mut digest);
        }
        Ok(CompleteRandomizationReferenceDigest(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), CompleteRandomizationError> {
        if self.reference_version != COMPLETE_RANDOMIZATION_REFERENCE_VERSION {
            return Err(CompleteRandomizationError::UnsupportedReferenceVersion(
                self.reference_version,
            ));
        }
        if self.estimand
            != RandomizationCausalEstimand::IntentionToTreatBinaryViabilityRiskDifference
        {
            return Err(CompleteRandomizationError::UnsupportedEstimand);
        }
        if self.assignment_unit_policy != AssignmentUnitPolicy::PersistentIndividual {
            return Err(CompleteRandomizationError::UnsupportedAssignmentUnitPolicy);
        }
        if self.support_policy != AssignmentSupportPolicy::CompleteFixedComparisonCount {
            return Err(CompleteRandomizationError::UnsupportedSupportPolicy);
        }
        if self.source_design_class != SelectionDesignClass::RandomizedInterventional {
            return Err(CompleteRandomizationError::UnsupportedDesignClass(
                self.source_design_class,
            ));
        }
        if self.causal_target != CausalSelectionTarget::BinaryViabilityRiskDifference {
            return Err(CompleteRandomizationError::UnsupportedTarget);
        }
        if self.estimand_denominator == 0
            || self.estimand_denominator > self.original_denominator
            || self.realized_assignments.len() as u64 != self.estimand_denominator
        {
            return Err(CompleteRandomizationError::DenominatorInvariant);
        }
        if self.estimand_denominator > COMPLETE_RANDOMIZATION_MAX_UNITS {
            return Err(CompleteRandomizationError::ReferencePopulationTooLarge {
                units: self.estimand_denominator,
                max_units: COMPLETE_RANDOMIZATION_MAX_UNITS,
            });
        }
        if self
            .realized_assignments
            .windows(2)
            .any(|window| window[0].individual_id >= window[1].individual_id)
        {
            return Err(CompleteRandomizationError::NonCanonicalAssignmentOrder);
        }

        let mut reference_count = 0u64;
        let mut comparison_count = 0u64;
        for assignment in &self.realized_assignments {
            match assignment.group {
                BinaryComparisonGroup::Reference => {
                    reference_count = reference_count
                        .checked_add(1)
                        .ok_or(CompleteRandomizationError::ArithmeticOverflow)?;
                }
                BinaryComparisonGroup::Comparison => {
                    comparison_count = comparison_count
                        .checked_add(1)
                        .ok_or(CompleteRandomizationError::ArithmeticOverflow)?;
                }
            }
        }
        if reference_count == 0 {
            return Err(CompleteRandomizationError::EmptyReferenceGroup);
        }
        if comparison_count == 0 {
            return Err(CompleteRandomizationError::EmptyComparisonGroup);
        }
        if reference_count != self.reference_count
            || comparison_count != self.comparison_count
            || reference_count
                .checked_add(comparison_count)
                .ok_or(CompleteRandomizationError::ArithmeticOverflow)?
                != self.estimand_denominator
        {
            return Err(CompleteRandomizationError::AssignmentCountInvariant);
        }
        if choose_u64(self.estimand_denominator, self.comparison_count)? != self.support_size {
            return Err(CompleteRandomizationError::SupportSizeInvariant);
        }

        validate_criterion_field(
            &self.assignment_mechanism,
            CausalIdentificationCriterion::AssignmentMechanism,
            self.design_digest,
            self.frame_digest,
        )?;
        validate_criterion_field(
            &self.allocation_integrity,
            CausalIdentificationCriterion::AllocationIntegrity,
            self.design_digest,
            self.frame_digest,
        )?;
        validate_criterion_field(
            &self.temporal_precedence,
            CausalIdentificationCriterion::TemporalPrecedence,
            self.design_digest,
            self.frame_digest,
        )?;
        validate_criterion_field(
            &self.intervention_fidelity,
            CausalIdentificationCriterion::InterventionFidelity,
            self.design_digest,
            self.frame_digest,
        )?;
        validate_criterion_field(
            &self.interference_policy,
            CausalIdentificationCriterion::InterferencePolicy,
            self.design_digest,
            self.frame_digest,
        )?;
        validate_criterion_field(
            &self.analysis_population_integrity,
            CausalIdentificationCriterion::AnalysisPopulationIntegrity,
            self.design_digest,
            self.frame_digest,
        )?;
        validate_criterion_field(
            &self.protocol_freeze_provenance,
            CausalIdentificationCriterion::ProtocolFreezeProvenance,
            self.design_digest,
            self.frame_digest,
        )?;
        self.mechanism_materialization
            .validate_subject(self.identification_digest)?;
        self.individual_exchangeability
            .validate_subject(self.identification_digest)?;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompleteRandomizationReferenceDigest([u8; 32]);

impl CompleteRandomizationReferenceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for CompleteRandomizationReferenceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CompleteRandomizationReferenceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for CompleteRandomizationReferenceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated randomization-reference authority should gate causal randomization tests"]
pub struct ValidatedCompleteRandomizationReference<'a> {
    reference: &'a CompleteRandomizationReferenceModel,
    reference_digest: CompleteRandomizationReferenceDigest,
    identification_digest: CausalSelectionIdentificationDigest,
    design_digest: SelectionComparisonDesignDigest,
    frame_digest: ExplicitSelectionAnalysisFrameDigest,
}

impl<'a> ValidatedCompleteRandomizationReference<'a> {
    pub fn validate_current(
        reference: &'a CompleteRandomizationReferenceModel,
        frame: &ValidatedSelectionAnalysisFrame<'_>,
        identification: &ValidatedCausalSelectionIdentification<'_>,
        mechanism_materialization: AssignmentMechanismMaterializationRef,
        individual_exchangeability: IndividualAssignmentExchangeabilityRef,
    ) -> Result<Self, CompleteRandomizationError> {
        reference.validate_current(
            frame,
            identification,
            mechanism_materialization,
            individual_exchangeability,
        )?;
        Ok(Self {
            reference,
            reference_digest: reference.canonical_digest()?,
            identification_digest: identification.identification_digest(),
            design_digest: frame.design_digest(),
            frame_digest: frame.frame_digest(),
        })
    }

    pub fn reference(&self) -> &'a CompleteRandomizationReferenceModel {
        self.reference
    }

    pub fn reference_digest(&self) -> CompleteRandomizationReferenceDigest {
        self.reference_digest
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
}

fn validate_capability_chain(
    frame: &ValidatedSelectionAnalysisFrame<'_>,
    identification: &ValidatedCausalSelectionIdentification<'_>,
) -> Result<(), CompleteRandomizationError> {
    if frame.design_class() != SelectionDesignClass::RandomizedInterventional {
        return Err(CompleteRandomizationError::UnsupportedDesignClass(
            frame.design_class(),
        ));
    }
    if frame.estimand() != SelectionEstimand::ViabilityWindowRiskContrast {
        return Err(CompleteRandomizationError::UnsupportedFrameEstimand(
            frame.estimand(),
        ));
    }
    if frame.frame().predictor_representation != PredictorRepresentation::BinaryComparisonGroup {
        return Err(CompleteRandomizationError::UnsupportedPredictorRepresentation);
    }
    if identification.tier() != CausalIdentificationTier::InterventionIdentified {
        return Err(CompleteRandomizationError::UnsupportedIdentificationTier);
    }
    if identification.target() != CausalSelectionTarget::BinaryViabilityRiskDifference {
        return Err(CompleteRandomizationError::UnsupportedTarget);
    }
    if identification.frame_digest() != frame.frame_digest() {
        return Err(CompleteRandomizationError::IdentificationFrameMismatch);
    }
    if identification.design_digest() != frame.design_digest() {
        return Err(CompleteRandomizationError::IdentificationDesignMismatch);
    }
    if identification.identification().source_design_class
        != SelectionDesignClass::RandomizedInterventional
    {
        return Err(CompleteRandomizationError::UnsupportedDesignClass(
            identification.identification().source_design_class,
        ));
    }
    if identification.identification().original_denominator()
        != frame.frame().original_denominator()
        || identification.identification().estimand_denominator()
            != frame.frame().estimand_denominator()
    {
        return Err(CompleteRandomizationError::DenominatorInvariant);
    }
    Ok(())
}

fn criterion_record(
    identification: &CausalSelectionIdentification,
    criterion: CausalIdentificationCriterion,
) -> Result<IdentificationEvidenceRef, CompleteRandomizationError> {
    identification
        .criteria
        .iter()
        .find(|item| item.criterion == criterion)
        .cloned()
        .ok_or(CompleteRandomizationError::MissingCriterion(criterion))
}

fn validate_criterion_field(
    record: &IdentificationEvidenceRef,
    expected: CausalIdentificationCriterion,
    design_digest: SelectionComparisonDesignDigest,
    frame_digest: ExplicitSelectionAnalysisFrameDigest,
) -> Result<(), CompleteRandomizationError> {
    if record.criterion != expected {
        return Err(CompleteRandomizationError::CriterionTypeMismatch {
            expected,
            actual: record.criterion,
        });
    }
    if record.subject_design_digest != design_digest || record.subject_frame_digest != frame_digest {
        return Err(CompleteRandomizationError::CriterionSubjectMismatch(expected));
    }
    Ok(())
}

fn choose_u64(n: u64, k: u64) -> Result<u64, CompleteRandomizationError> {
    if k > n {
        return Err(CompleteRandomizationError::ArithmeticInvariant);
    }
    let k = k.min(n - k);
    let mut value = 1u128;
    for i in 0..k {
        value = value
            .checked_mul(u128::from(n - i))
            .ok_or(CompleteRandomizationError::ArithmeticOverflow)?;
        value /= u128::from(i + 1);
    }
    u64::try_from(value).map_err(|_| CompleteRandomizationError::ArithmeticOverflow)
}

fn update_criterion_digest(digest: &mut Sha256, record: &IdentificationEvidenceRef) {
    digest.update([criterion_tag(record.criterion)]);
    put_text(digest, record.authority_id.as_str());
    put_u64(digest, record.revision);
    digest.update(record.content_digest.as_bytes());
    put_text(digest, record.qualification.authority_id.as_str());
    put_u64(digest, record.qualification.revision);
    digest.update(record.qualification.content_digest.as_bytes());
    digest.update(record.subject_design_digest.as_bytes());
    digest.update(record.subject_frame_digest.as_bytes());
}

fn criterion_tag(value: CausalIdentificationCriterion) -> u8 {
    match value {
        CausalIdentificationCriterion::AssignmentMechanism => 0,
        CausalIdentificationCriterion::AllocationIntegrity => 1,
        CausalIdentificationCriterion::TemporalPrecedence => 2,
        CausalIdentificationCriterion::InterventionFidelity => 3,
        CausalIdentificationCriterion::InterferencePolicy => 4,
        CausalIdentificationCriterion::OutcomeAscertainment => 5,
        CausalIdentificationCriterion::AttritionMissingness => 6,
        CausalIdentificationCriterion::AnalysisPopulationIntegrity => 7,
        CausalIdentificationCriterion::ProtocolFreezeProvenance => 8,
        CausalIdentificationCriterion::SimulationReplayIdentity => 9,
        CausalIdentificationCriterion::InterventionIsolation => 10,
        CausalIdentificationCriterion::ScenarioContext => 11,
        CausalIdentificationCriterion::SimulationStochasticityPolicy => 12,
    }
}

fn group_tag(value: BinaryComparisonGroup) -> u8 {
    match value {
        BinaryComparisonGroup::Reference => 0,
        BinaryComparisonGroup::Comparison => 1,
    }
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

fn target_tag(value: CausalSelectionTarget) -> u8 {
    match value {
        CausalSelectionTarget::BinaryViabilityRiskDifference => 0,
    }
}

#[derive(Debug)]
pub enum CompleteRandomizationError {
    UnsupportedReferenceVersion(u32),
    UnsupportedEstimand,
    UnsupportedAssignmentUnitPolicy,
    UnsupportedSupportPolicy,
    UnsupportedDesignClass(SelectionDesignClass),
    UnsupportedIdentificationTier,
    UnsupportedTarget,
    UnsupportedFrameEstimand(SelectionEstimand),
    UnsupportedPredictorRepresentation,
    IdentificationFrameMismatch,
    IdentificationDesignMismatch,
    MaterializationSubjectMismatch,
    ExchangeabilitySubjectMismatch,
    MissingCriterion(CausalIdentificationCriterion),
    CriterionTypeMismatch {
        expected: CausalIdentificationCriterion,
        actual: CausalIdentificationCriterion,
    },
    CriterionSubjectMismatch(CausalIdentificationCriterion),
    UnsupportedRealizedAssignment(EvolutionIndividualId),
    EmptyReferenceGroup,
    EmptyComparisonGroup,
    DenominatorInvariant,
    NonCanonicalAssignmentOrder,
    AssignmentCountInvariant,
    ReferencePopulationTooLarge { units: u64, max_units: u64 },
    SupportSizeInvariant,
    ArithmeticOverflow,
    ArithmeticInvariant,
    ReferenceReplayMismatch,
}

impl fmt::Display for CompleteRandomizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedReferenceVersion(version) => {
                write!(f, "unsupported complete-randomization reference version {version}")
            }
            Self::UnsupportedEstimand => write!(
                f,
                "SEL-07D1 V1 supports only intention-to-treat binary viability-risk difference"
            ),
            Self::UnsupportedAssignmentUnitPolicy => write!(
                f,
                "SEL-07D1 V1 supports only persistent-individual assignment units"
            ),
            Self::UnsupportedSupportPolicy => write!(
                f,
                "SEL-07D1 V1 supports only complete fixed-comparison-count assignment support"
            ),
            Self::UnsupportedDesignClass(class) => write!(
                f,
                "SEL-07D1 V1 requires randomized-interventional design, got {class:?}"
            ),
            Self::UnsupportedIdentificationTier => write!(
                f,
                "SEL-07D1 V1 requires current intervention-identified causal authority"
            ),
            Self::UnsupportedTarget => write!(
                f,
                "SEL-07D1 V1 requires the binary viability-risk causal target"
            ),
            Self::UnsupportedFrameEstimand(estimand) => write!(
                f,
                "SEL-07D1 V1 requires viability-window risk contrast, got {estimand:?}"
            ),
            Self::UnsupportedPredictorRepresentation => write!(
                f,
                "SEL-07D1 V1 requires binary Reference/Comparison assignment materialization"
            ),
            Self::IdentificationFrameMismatch => {
                write!(f, "causal identification is bound to another analysis frame")
            }
            Self::IdentificationDesignMismatch => {
                write!(f, "causal identification is bound to another comparison design")
            }
            Self::MaterializationSubjectMismatch => write!(
                f,
                "assignment-mechanism materialization authority is bound to another identification"
            ),
            Self::ExchangeabilitySubjectMismatch => write!(
                f,
                "individual-exchangeability authority is bound to another identification"
            ),
            Self::MissingCriterion(criterion) => {
                write!(f, "validated causal identification lacks required criterion {criterion:?}")
            }
            Self::CriterionTypeMismatch { expected, actual } => write!(
                f,
                "randomization reference criterion field expected {expected:?}, got {actual:?}"
            ),
            Self::CriterionSubjectMismatch(criterion) => write!(
                f,
                "randomization reference criterion {criterion:?} is bound to another design/frame"
            ),
            Self::UnsupportedRealizedAssignment(id) => write!(
                f,
                "individual {id:?} lacks an observed binary realized assignment"
            ),
            Self::EmptyReferenceGroup => write!(f, "complete-randomization reference group is empty"),
            Self::EmptyComparisonGroup => write!(f, "complete-randomization comparison group is empty"),
            Self::DenominatorInvariant => write!(f, "complete-randomization denominator invariant failed"),
            Self::NonCanonicalAssignmentOrder => write!(
                f,
                "complete-randomization assignment units are not in canonical persistent-ID order"
            ),
            Self::AssignmentCountInvariant => {
                write!(f, "realized assignment counts do not match stored group counts")
            }
            Self::ReferencePopulationTooLarge { units, max_units } => write!(
                f,
                "complete-randomization exact support allows at most {max_units} units, got {units}"
            ),
            Self::SupportSizeInvariant => write!(
                f,
                "stored complete-randomization support size does not equal C(N, comparison_count)"
            ),
            Self::ArithmeticOverflow => write!(f, "complete-randomization exact arithmetic overflowed"),
            Self::ArithmeticInvariant => write!(f, "complete-randomization arithmetic invariant failed"),
            Self::ReferenceReplayMismatch => write!(
                f,
                "persisted complete-randomization reference does not replay against current authorities"
            ),
        }
    }
}

impl Error for CompleteRandomizationError {}
