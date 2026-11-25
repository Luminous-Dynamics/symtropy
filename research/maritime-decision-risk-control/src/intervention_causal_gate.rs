// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Intervention exposure and causal-claim admission for maritime recovery.
//!
//! The ledger keeps announcement, commitment, availability, activation and
//! realized resource output separate. A published plan is not treatment
//! exposure; a commissioning announcement is not productive capacity.
//!
//! The causal gate does not estimate effects. It decides whether an evaluation
//! has enough declared design/evidence to be sent to an independent estimator.
//! Missing evidence, bundled interventions, weak pre-trends, poor overlap,
//! unmodelled network spillovers or incomplete receiver outcomes yield
//! NOT_IDENTIFIED. References are caller-declared pointers, not authenticated
//! proof that assumptions hold.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum InterventionStage {
    Announced,
    Committed,
    Available,
    Activated,
    Realized,
    Withdrawn,
}

impl InterventionStage {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Announced => "ANNOUNCED",
            Self::Committed => "SCHEDULED_COMMITTED",
            Self::Available => "AVAILABLE",
            Self::Activated => "ACTIVATED",
            Self::Realized => "REALIZED",
            Self::Withdrawn => "WITHDRAWN",
        }
    }

    const fn ordinal(self) -> Option<u8> {
        match self {
            Self::Announced => Some(0),
            Self::Committed => Some(1),
            Self::Available => Some(2),
            Self::Activated => Some(3),
            Self::Realized => Some(4),
            Self::Withdrawn => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterventionExposureEvent {
    pub intervention_id: String,
    /// Resource or cargo unit whose treatment exposure is being tracked.
    pub exposed_unit_id: String,
    pub resource_id: String,
    pub stage: InterventionStage,
    /// Business/event time, not publication or repository retrieval time.
    pub occurred_at_unix: i64,
    pub published_at_unix: i64,
    pub retrieved_at_unix: i64,
    pub source_record_id: String,
    pub evidence_ref: String,
    /// Required only for REALIZED; e.g. productive equipment hours or delivered
    /// resource quantity, in the unit declared by the intervention contract.
    pub realized_quantity_milli: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExposureLedgerError {
    EmptyIdentity,
    NegativeTimestamp,
    PublicationAfterRetrieval,
    EvidenceNotAvailableAtAsOf,
    EventAfterAsOf,
    RealizedQuantityMissing,
    RealizedQuantityMustBePositive,
    UnexpectedRealizedQuantity,
    FirstStageMustBeAnnounced,
    StageTransitionSkippedOrReversed,
    EventTimeWentBackwards,
    ExposureAlreadyTerminal,
    DuplicateSourceRecord,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentExposure {
    pub intervention_id: String,
    pub exposed_unit_id: String,
    pub resource_id: String,
    pub stage: InterventionStage,
    pub latest_event_time_unix: i64,
    pub realized_quantity_milli: Option<u64>,
    pub source_record_id: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InterventionExposureLedger {
    events: Vec<InterventionExposureEvent>,
}

impl InterventionExposureLedger {
    pub fn events(&self) -> &[InterventionExposureEvent] {
        &self.events
    }

    /// Appends a strictly ordered lifecycle event. No update/delete operation
    /// exists: corrections must arrive as new source records and be reconciled.
    pub fn append(
        &mut self,
        event: InterventionExposureEvent,
        as_of_unix: i64,
    ) -> Result<(), ExposureLedgerError> {
        if event.intervention_id.trim().is_empty()
            || event.exposed_unit_id.trim().is_empty()
            || event.resource_id.trim().is_empty()
            || event.source_record_id.trim().is_empty()
            || event.evidence_ref.trim().is_empty()
        {
            return Err(ExposureLedgerError::EmptyIdentity);
        }
        if event.occurred_at_unix < 0
            || event.published_at_unix < 0
            || event.retrieved_at_unix < 0
            || as_of_unix < 0
        {
            return Err(ExposureLedgerError::NegativeTimestamp);
        }
        if event.published_at_unix > event.retrieved_at_unix {
            return Err(ExposureLedgerError::PublicationAfterRetrieval);
        }
        if event.published_at_unix > as_of_unix || event.retrieved_at_unix > as_of_unix {
            return Err(ExposureLedgerError::EvidenceNotAvailableAtAsOf);
        }
        if event.occurred_at_unix > as_of_unix {
            return Err(ExposureLedgerError::EventAfterAsOf);
        }
        match (event.stage, event.realized_quantity_milli) {
            (InterventionStage::Realized, None) => {
                return Err(ExposureLedgerError::RealizedQuantityMissing);
            }
            (InterventionStage::Realized, Some(0)) => {
                return Err(ExposureLedgerError::RealizedQuantityMustBePositive);
            }
            (InterventionStage::Realized, Some(_)) => {}
            (_, Some(_)) => return Err(ExposureLedgerError::UnexpectedRealizedQuantity),
            (_, None) => {}
        }
        if self
            .events
            .iter()
            .any(|existing| existing.source_record_id == event.source_record_id)
        {
            return Err(ExposureLedgerError::DuplicateSourceRecord);
        }

        let mut matching = self.events.iter().filter(|existing| {
            existing.intervention_id == event.intervention_id
                && existing.exposed_unit_id == event.exposed_unit_id
                && existing.resource_id == event.resource_id
        });
        let previous = matching.next_back();
        match previous {
            None if event.stage != InterventionStage::Announced => {
                return Err(ExposureLedgerError::FirstStageMustBeAnnounced);
            }
            Some(previous) => {
                if previous.stage == InterventionStage::Realized
                    || previous.stage == InterventionStage::Withdrawn
                {
                    return Err(ExposureLedgerError::ExposureAlreadyTerminal);
                }
                if event.occurred_at_unix < previous.occurred_at_unix {
                    return Err(ExposureLedgerError::EventTimeWentBackwards);
                }
                let valid_next = if event.stage == InterventionStage::Withdrawn {
                    true
                } else {
                    previous.stage.ordinal().zip(event.stage.ordinal()).is_some_and(
                        |(previous_stage, next_stage)| next_stage == previous_stage + 1,
                    )
                };
                if !valid_next {
                    return Err(ExposureLedgerError::StageTransitionSkippedOrReversed);
                }
            }
            None => {}
        }

        self.events.push(event);
        Ok(())
    }

    pub fn current_exposure(
        &self,
        intervention_id: &str,
        exposed_unit_id: &str,
        resource_id: &str,
        as_of_unix: i64,
    ) -> Option<CurrentExposure> {
        self.events
            .iter()
            .filter(|event| {
                event.intervention_id == intervention_id
                    && event.exposed_unit_id == exposed_unit_id
                    && event.resource_id == resource_id
                    && event.published_at_unix <= as_of_unix
                    && event.retrieved_at_unix <= as_of_unix
                    && event.occurred_at_unix <= as_of_unix
            })
            .last()
            .map(|event| CurrentExposure {
                intervention_id: event.intervention_id.clone(),
                exposed_unit_id: event.exposed_unit_id.clone(),
                resource_id: event.resource_id.clone(),
                stage: event.stage,
                latest_event_time_unix: event.occurred_at_unix,
                realized_quantity_milli: event.realized_quantity_milli,
                source_record_id: event.source_record_id.clone(),
            })
    }

    pub fn unit_is_realized(
        &self,
        intervention_id: &str,
        exposed_unit_id: &str,
        as_of_unix: i64,
    ) -> bool {
        self.events.iter().any(|event| {
            event.intervention_id == intervention_id
                && event.exposed_unit_id == exposed_unit_id
                && event.stage == InterventionStage::Realized
                && event.published_at_unix <= as_of_unix
                && event.retrieved_at_unix <= as_of_unix
                && event.occurred_at_unix <= as_of_unix
        })
    }

    pub fn unit_was_activated(
        &self,
        intervention_id: &str,
        exposed_unit_id: &str,
        as_of_unix: i64,
    ) -> bool {
        self.events.iter().any(|event| {
            event.intervention_id == intervention_id
                && event.exposed_unit_id == exposed_unit_id
                && matches!(event.stage, InterventionStage::Activated | InterventionStage::Realized)
                && event.published_at_unix <= as_of_unix
                && event.retrieved_at_unix <= as_of_unix
                && event.occurred_at_unix <= as_of_unix
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CausalDesign {
    DescriptiveBeforeAfter,
    RandomizedAssignment,
    DifferenceInDifferences,
    SyntheticControl,
    InterruptedTimeSeries,
}

impl CausalDesign {
    pub const fn label(self) -> &'static str {
        match self {
            Self::DescriptiveBeforeAfter => "DESCRIPTIVE_BEFORE_AFTER",
            Self::RandomizedAssignment => "RANDOMIZED_ASSIGNMENT",
            Self::DifferenceInDifferences => "DIFFERENCE_IN_DIFFERENCES",
            Self::SyntheticControl => "SYNTHETIC_CONTROL",
            Self::InterruptedTimeSeries => "INTERRUPTED_TIME_SERIES",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComparisonEvidence {
    RandomizedAssignmentVerified,
    ConcurrentMatchedControl,
    NoCredibleComparison,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreTrendEvidence {
    Passed,
    Failed,
    NotAssessed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverlapEvidence {
    Adequate,
    Inadequate,
    NotAssessed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterferenceEvidence {
    NetworkSpilloversModelled,
    EvidenceOfNoMaterialInterference,
    UnmodelledSpillovers,
    NotAssessed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfoundingEvidence {
    TimeVaryingConfoundersAddressed,
    SensitivityAnalysisCompleted,
    UnaddressedTimeVaryingConfounding,
    NotAssessed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoInterventionEvidence {
    RegisteredAndModelled,
    UnregisteredCoInterventions,
    NotAssessed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutcomeEvidence {
    CompleteReceiverQualifiedOutcomes,
    PartialOutcomeCapture,
    UnknownOutcomeCapture,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TreatmentTimingEvidence {
    UnitLevelExposureTimestamps,
    AggregateOnlyTiming,
    NoTreatmentTiming,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CausalEvidenceReferences {
    pub design_protocol_ref: String,
    pub comparison_group_ref: String,
    pub pretrend_diagnostic_ref: String,
    pub overlap_diagnostic_ref: String,
    pub interference_review_ref: String,
    pub confounder_register_ref: String,
    pub co_intervention_register_ref: String,
    pub outcome_capture_ref: String,
    pub treatment_exposure_ref: String,
    pub regime_definition_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CausalIdentificationAssessment {
    pub intervention_id: String,
    pub design: CausalDesign,
    pub comparison: ComparisonEvidence,
    pub pretrend: PreTrendEvidence,
    pub overlap: OverlapEvidence,
    pub interference: InterferenceEvidence,
    pub confounding: ConfoundingEvidence,
    pub co_interventions: CoInterventionEvidence,
    pub outcomes: OutcomeEvidence,
    pub treatment_timing: TreatmentTimingEvidence,
    pub treatment_unit_ids: Vec<String>,
    pub comparison_unit_ids: Vec<String>,
    pub regime_id: String,
    pub pre_period_start_unix: i64,
    pub pre_period_end_unix: i64,
    pub post_period_start_unix: i64,
    pub post_period_end_unix: i64,
    pub as_of_unix: i64,
    pub evidence: CausalEvidenceReferences,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CausalBlocker {
    DescriptiveDesignOnly,
    EmptyInterventionIdentity,
    EmptyRegimeIdentity,
    EmptyTreatmentUnits,
    EmptyComparisonUnits,
    TreatmentComparisonUnitsOverlap,
    InvalidPeriodOrdering,
    MissingEvidenceReference,
    RandomizationNotVerified,
    ConcurrentComparisonMissing,
    PreTrendFailed,
    PreTrendNotAssessed,
    OverlapInadequate,
    OverlapNotAssessed,
    SpilloversUnmodelled,
    InterferenceNotAssessed,
    TimeVaryingConfoundingUnaddressed,
    ConfoundingNotAssessed,
    CoInterventionsUnregistered,
    CoInterventionsNotAssessed,
    ReceiverOutcomesIncomplete,
    ReceiverOutcomeCaptureUnknown,
    TreatmentTimingNotUnitLevel,
    TreatmentExposureNotRealized,
    ComparisonUnitContaminated,
    RegimeNotMatched,
}

impl CausalBlocker {
    pub const fn label(self) -> &'static str {
        match self {
            Self::DescriptiveDesignOnly => "DESCRIPTIVE_DESIGN_ONLY",
            Self::EmptyInterventionIdentity => "EMPTY_INTERVENTION_ID",
            Self::EmptyRegimeIdentity => "EMPTY_REGIME_ID",
            Self::EmptyTreatmentUnits => "EMPTY_TREATMENT_UNITS",
            Self::EmptyComparisonUnits => "EMPTY_COMPARISON_UNITS",
            Self::TreatmentComparisonUnitsOverlap => "TREATMENT_COMPARISON_OVERLAP",
            Self::InvalidPeriodOrdering => "INVALID_PERIOD_ORDERING",
            Self::MissingEvidenceReference => "MISSING_EVIDENCE_REFERENCE",
            Self::RandomizationNotVerified => "RANDOMIZATION_NOT_VERIFIED",
            Self::ConcurrentComparisonMissing => "CONCURRENT_COMPARISON_MISSING",
            Self::PreTrendFailed => "PRETREND_FAILED",
            Self::PreTrendNotAssessed => "PRETREND_NOT_ASSESSED",
            Self::OverlapInadequate => "OVERLAP_INADEQUATE",
            Self::OverlapNotAssessed => "OVERLAP_NOT_ASSESSED",
            Self::SpilloversUnmodelled => "SPILLOVERS_UNMODELLED",
            Self::InterferenceNotAssessed => "INTERFERENCE_NOT_ASSESSED",
            Self::TimeVaryingConfoundingUnaddressed => "TIME_VARYING_CONFOUNDING_UNADDRESSED",
            Self::ConfoundingNotAssessed => "CONFOUNDING_NOT_ASSESSED",
            Self::CoInterventionsUnregistered => "CO_INTERVENTIONS_UNREGISTERED",
            Self::CoInterventionsNotAssessed => "CO_INTERVENTIONS_NOT_ASSESSED",
            Self::ReceiverOutcomesIncomplete => "RECEIVER_OUTCOMES_INCOMPLETE",
            Self::ReceiverOutcomeCaptureUnknown => "RECEIVER_OUTCOME_CAPTURE_UNKNOWN",
            Self::TreatmentTimingNotUnitLevel => "TREATMENT_TIMING_NOT_UNIT_LEVEL",
            Self::TreatmentExposureNotRealized => "TREATMENT_EXPOSURE_NOT_REALIZED",
            Self::ComparisonUnitContaminated => "COMPARISON_UNIT_CONTAMINATED",
            Self::RegimeNotMatched => "REGIME_NOT_MATCHED",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CausalIdentificationDecision {
    /// A design is eligible for review by an independent causal estimator.
    /// This is not an identified effect or proof that declared assumptions hold.
    EligibleForIndependentEstimator {
        intervention_id: String,
        design: CausalDesign,
        treatment_units: usize,
        comparison_units: usize,
        regime_id: String,
        claim_ceiling: &'static str,
    },
    NotIdentified {
        blockers: Vec<CausalBlocker>,
        claim_ceiling: &'static str,
    },
    DescriptiveOnly {
        claim_ceiling: &'static str,
    },
}

impl CausalIdentificationDecision {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::EligibleForIndependentEstimator { .. } => "ELIGIBLE_FOR_INDEPENDENT_ESTIMATOR",
            Self::NotIdentified { .. } => "NOT_IDENTIFIED",
            Self::DescriptiveOnly { .. } => "DESCRIPTIVE_ONLY",
        }
    }

    pub fn blocker_labels(&self) -> String {
        match self {
            Self::NotIdentified { blockers, .. } => blockers
                .iter()
                .map(|blocker| blocker.label())
                .collect::<Vec<_>>()
                .join("|"),
            _ => "NONE".to_owned(),
        }
    }
}

fn evidence_references_present(evidence: &CausalEvidenceReferences) -> bool {
    [
        evidence.design_protocol_ref.as_str(),
        evidence.comparison_group_ref.as_str(),
        evidence.pretrend_diagnostic_ref.as_str(),
        evidence.overlap_diagnostic_ref.as_str(),
        evidence.interference_review_ref.as_str(),
        evidence.confounder_register_ref.as_str(),
        evidence.co_intervention_register_ref.as_str(),
        evidence.outcome_capture_ref.as_str(),
        evidence.treatment_exposure_ref.as_str(),
        evidence.regime_definition_ref.as_str(),
    ]
    .iter()
    .all(|reference| !reference.trim().is_empty())
}

/// Checks identification prerequisites without estimating or reporting an
/// intervention effect. Evidence references remain caller-declared.
pub fn assess_causal_identification(
    ledger: &InterventionExposureLedger,
    assessment: &CausalIdentificationAssessment,
) -> CausalIdentificationDecision {
    if assessment.design == CausalDesign::DescriptiveBeforeAfter {
        return CausalIdentificationDecision::DescriptiveOnly {
            claim_ceiling: "DESCRIPTIVE_SOURCE_QUALIFIED_BEFORE_AFTER_ONLY",
        };
    }

    let mut blockers = Vec::new();
    if assessment.intervention_id.trim().is_empty() {
        blockers.push(CausalBlocker::EmptyInterventionIdentity);
    }
    if assessment.regime_id.trim().is_empty() {
        blockers.push(CausalBlocker::EmptyRegimeIdentity);
    }
    if assessment.treatment_unit_ids.is_empty() {
        blockers.push(CausalBlocker::EmptyTreatmentUnits);
    }
    if assessment.comparison_unit_ids.is_empty() {
        blockers.push(CausalBlocker::EmptyComparisonUnits);
    }
    if assessment
        .treatment_unit_ids
        .iter()
        .any(|unit| assessment.comparison_unit_ids.contains(unit))
    {
        blockers.push(CausalBlocker::TreatmentComparisonUnitsOverlap);
    }
    if assessment.pre_period_start_unix < 0
        || assessment.pre_period_end_unix <= assessment.pre_period_start_unix
        || assessment.post_period_start_unix <= assessment.pre_period_end_unix
        || assessment.post_period_end_unix <= assessment.post_period_start_unix
        || assessment.as_of_unix < assessment.post_period_end_unix
    {
        blockers.push(CausalBlocker::InvalidPeriodOrdering);
    }
    if !evidence_references_present(&assessment.evidence) {
        blockers.push(CausalBlocker::MissingEvidenceReference);
    }

    match assessment.design {
        CausalDesign::RandomizedAssignment => {
            if assessment.comparison != ComparisonEvidence::RandomizedAssignmentVerified {
                blockers.push(CausalBlocker::RandomizationNotVerified);
            }
        }
        CausalDesign::DifferenceInDifferences | CausalDesign::SyntheticControl => {
            if assessment.comparison != ComparisonEvidence::ConcurrentMatchedControl {
                blockers.push(CausalBlocker::ConcurrentComparisonMissing);
            }
            match assessment.pretrend {
                PreTrendEvidence::Passed => {}
                PreTrendEvidence::Failed => blockers.push(CausalBlocker::PreTrendFailed),
                PreTrendEvidence::NotAssessed => blockers.push(CausalBlocker::PreTrendNotAssessed),
            }
        }
        CausalDesign::InterruptedTimeSeries => match assessment.pretrend {
            PreTrendEvidence::Passed => {}
            PreTrendEvidence::Failed => blockers.push(CausalBlocker::PreTrendFailed),
            PreTrendEvidence::NotAssessed => blockers.push(CausalBlocker::PreTrendNotAssessed),
        },
        CausalDesign::DescriptiveBeforeAfter => unreachable!("handled above"),
    }

    match assessment.overlap {
        OverlapEvidence::Adequate => {}
        OverlapEvidence::Inadequate => blockers.push(CausalBlocker::OverlapInadequate),
        OverlapEvidence::NotAssessed => blockers.push(CausalBlocker::OverlapNotAssessed),
    }
    match assessment.interference {
        InterferenceEvidence::NetworkSpilloversModelled
        | InterferenceEvidence::EvidenceOfNoMaterialInterference => {}
        InterferenceEvidence::UnmodelledSpillovers => blockers.push(CausalBlocker::SpilloversUnmodelled),
        InterferenceEvidence::NotAssessed => blockers.push(CausalBlocker::InterferenceNotAssessed),
    }
    match assessment.confounding {
        ConfoundingEvidence::TimeVaryingConfoundersAddressed
        | ConfoundingEvidence::SensitivityAnalysisCompleted => {}
        ConfoundingEvidence::UnaddressedTimeVaryingConfounding => {
            blockers.push(CausalBlocker::TimeVaryingConfoundingUnaddressed);
        }
        ConfoundingEvidence::NotAssessed => blockers.push(CausalBlocker::ConfoundingNotAssessed),
    }
    match assessment.co_interventions {
        CoInterventionEvidence::RegisteredAndModelled => {}
        CoInterventionEvidence::UnregisteredCoInterventions => {
            blockers.push(CausalBlocker::CoInterventionsUnregistered);
        }
        CoInterventionEvidence::NotAssessed => blockers.push(CausalBlocker::CoInterventionsNotAssessed),
    }
    match assessment.outcomes {
        OutcomeEvidence::CompleteReceiverQualifiedOutcomes => {}
        OutcomeEvidence::PartialOutcomeCapture => blockers.push(CausalBlocker::ReceiverOutcomesIncomplete),
        OutcomeEvidence::UnknownOutcomeCapture => blockers.push(CausalBlocker::ReceiverOutcomeCaptureUnknown),
    }
    if assessment.treatment_timing != TreatmentTimingEvidence::UnitLevelExposureTimestamps {
        blockers.push(CausalBlocker::TreatmentTimingNotUnitLevel);
    }

    for unit_id in &assessment.treatment_unit_ids {
        if !ledger.unit_is_realized(&assessment.intervention_id, unit_id, assessment.as_of_unix) {
            blockers.push(CausalBlocker::TreatmentExposureNotRealized);
            break;
        }
    }
    for unit_id in &assessment.comparison_unit_ids {
        if ledger.unit_was_activated(&assessment.intervention_id, unit_id, assessment.as_of_unix) {
            blockers.push(CausalBlocker::ComparisonUnitContaminated);
            break;
        }
    }

    if blockers.is_empty() {
        CausalIdentificationDecision::EligibleForIndependentEstimator {
            intervention_id: assessment.intervention_id.clone(),
            design: assessment.design,
            treatment_units: assessment.treatment_unit_ids.len(),
            comparison_units: assessment.comparison_unit_ids.len(),
            regime_id: assessment.regime_id.clone(),
            claim_ceiling: "ELIGIBLE_FOR_INDEPENDENT_ESTIMATOR_NOT_IDENTIFIED_EFFECT",
        }
    } else {
        CausalIdentificationDecision::NotIdentified {
            blockers,
            claim_ceiling: "DESCRIPTIVE_ONLY_NO_CAUSAL_EFFECT_CLAIM",
        }
    }
}

fn fixture_event(
    intervention_id: &str,
    unit_id: &str,
    stage: InterventionStage,
    offset: i64,
    source_record_id: &str,
    quantity: Option<u64>,
) -> InterventionExposureEvent {
    InterventionExposureEvent {
        intervention_id: intervention_id.to_owned(),
        exposed_unit_id: unit_id.to_owned(),
        resource_id: "BERTH-102".to_owned(),
        stage,
        occurred_at_unix: 1_800_000_000 + offset,
        published_at_unix: 1_800_000_100 + offset,
        retrieved_at_unix: 1_800_000_110 + offset,
        source_record_id: source_record_id.to_owned(),
        evidence_ref: format!("synthetic-evidence:{source_record_id}"),
        realized_quantity_milli: quantity,
    }
}

fn fixture_ledger(treatment_units: &[&str]) -> InterventionExposureLedger {
    let mut ledger = InterventionExposureLedger::default();
    for unit in treatment_units {
        for (index, stage) in [
            InterventionStage::Announced,
            InterventionStage::Committed,
            InterventionStage::Available,
            InterventionStage::Activated,
            InterventionStage::Realized,
        ]
        .iter()
        .enumerate()
        {
            ledger
                .append(
                    fixture_event(
                        "BERTH-102-DIRECT-DISCHARGE",
                        unit,
                        *stage,
                        index as i64 * 100,
                        &format!("{unit}-{stage:?}"),
                        (*stage == InterventionStage::Realized).then_some(30_000),
                    ),
                    1_900_000_000,
                )
                .expect("valid intervention exposure chain");
        }
    }
    ledger
}

fn fixture_evidence() -> CausalEvidenceReferences {
    CausalEvidenceReferences {
        design_protocol_ref: "synthetic:design-protocol".to_owned(),
        comparison_group_ref: "synthetic:concurrent-comparison".to_owned(),
        pretrend_diagnostic_ref: "synthetic:pretrend-report".to_owned(),
        overlap_diagnostic_ref: "synthetic:overlap-report".to_owned(),
        interference_review_ref: "synthetic:network-spillover-review".to_owned(),
        confounder_register_ref: "synthetic:time-varying-confounder-register".to_owned(),
        co_intervention_register_ref: "synthetic:co-intervention-register".to_owned(),
        outcome_capture_ref: "synthetic:receiver-outcome-capture".to_owned(),
        treatment_exposure_ref: "synthetic:intervention-exposure-ledger".to_owned(),
        regime_definition_ref: "synthetic:regime-definition".to_owned(),
    }
}

fn fixture_assessment() -> CausalIdentificationAssessment {
    CausalIdentificationAssessment {
        intervention_id: "BERTH-102-DIRECT-DISCHARGE".to_owned(),
        design: CausalDesign::DifferenceInDifferences,
        comparison: ComparisonEvidence::ConcurrentMatchedControl,
        pretrend: PreTrendEvidence::Passed,
        overlap: OverlapEvidence::Adequate,
        interference: InterferenceEvidence::NetworkSpilloversModelled,
        confounding: ConfoundingEvidence::TimeVaryingConfoundersAddressed,
        co_interventions: CoInterventionEvidence::RegisteredAndModelled,
        outcomes: OutcomeEvidence::CompleteReceiverQualifiedOutcomes,
        treatment_timing: TreatmentTimingEvidence::UnitLevelExposureTimestamps,
        treatment_unit_ids: vec!["TREATMENT-VISIT-001".to_owned(), "TREATMENT-VISIT-002".to_owned()],
        comparison_unit_ids: vec!["CONTROL-VISIT-001".to_owned(), "CONTROL-VISIT-002".to_owned()],
        regime_id: "DGT-AUGUST-INCIDENT-RECOVERY".to_owned(),
        pre_period_start_unix: 1_700_000_000,
        pre_period_end_unix: 1_750_000_000,
        post_period_start_unix: 1_760_000_000,
        post_period_end_unix: 1_800_000_000,
        as_of_unix: 1_900_000_000,
        evidence: fixture_evidence(),
    }
}

#[derive(Clone, Copy)]
struct GateCase {
    id: &'static str,
    design: CausalDesign,
    pretrend: PreTrendEvidence,
    spillover: InterferenceEvidence,
    outcomes: OutcomeEvidence,
    exposure_complete: bool,
    expected: &'static str,
}

fn gate_cases() -> [GateCase; 6] {
    [
        GateCase {
            id: "qualified_did_candidate_not_an_effect",
            design: CausalDesign::DifferenceInDifferences,
            pretrend: PreTrendEvidence::Passed,
            spillover: InterferenceEvidence::NetworkSpilloversModelled,
            outcomes: OutcomeEvidence::CompleteReceiverQualifiedOutcomes,
            exposure_complete: true,
            expected: "ELIGIBLE_FOR_INDEPENDENT_ESTIMATOR",
        },
        GateCase {
            id: "pretrend_failure_blocks_did",
            design: CausalDesign::DifferenceInDifferences,
            pretrend: PreTrendEvidence::Failed,
            spillover: InterferenceEvidence::NetworkSpilloversModelled,
            outcomes: OutcomeEvidence::CompleteReceiverQualifiedOutcomes,
            exposure_complete: true,
            expected: "NOT_IDENTIFIED",
        },
        GateCase {
            id: "unmodelled_terminal_spillovers_block",
            design: CausalDesign::DifferenceInDifferences,
            pretrend: PreTrendEvidence::Passed,
            spillover: InterferenceEvidence::UnmodelledSpillovers,
            outcomes: OutcomeEvidence::CompleteReceiverQualifiedOutcomes,
            exposure_complete: true,
            expected: "NOT_IDENTIFIED",
        },
        GateCase {
            id: "partial_receiver_capture_blocks",
            design: CausalDesign::DifferenceInDifferences,
            pretrend: PreTrendEvidence::Passed,
            spillover: InterferenceEvidence::NetworkSpilloversModelled,
            outcomes: OutcomeEvidence::PartialOutcomeCapture,
            exposure_complete: true,
            expected: "NOT_IDENTIFIED",
        },
        GateCase {
            id: "aggregate_announcement_only_is_not_treatment",
            design: CausalDesign::DifferenceInDifferences,
            pretrend: PreTrendEvidence::Passed,
            spillover: InterferenceEvidence::NetworkSpilloversModelled,
            outcomes: OutcomeEvidence::CompleteReceiverQualifiedOutcomes,
            exposure_complete: false,
            expected: "NOT_IDENTIFIED",
        },
        GateCase {
            id: "before_after_is_descriptive_only",
            design: CausalDesign::DescriptiveBeforeAfter,
            pretrend: PreTrendEvidence::NotAssessed,
            spillover: InterferenceEvidence::NotAssessed,
            outcomes: OutcomeEvidence::UnknownOutcomeCapture,
            exposure_complete: false,
            expected: "DESCRIPTIVE_ONLY",
        },
    ]
}

pub fn render_csv() -> String {
    let mut output = String::from(
        "case_id,design,pretrend,spillover,outcome_capture,exposure_state,decision,blockers,claim_ceiling\n",
    );
    for case in gate_cases() {
        let treatment_units = if case.exposure_complete {
            vec!["TREATMENT-VISIT-001", "TREATMENT-VISIT-002"]
        } else {
            vec!["TREATMENT-VISIT-001"]
        };
        let ledger = if case.exposure_complete {
            fixture_ledger(&treatment_units)
        } else {
            let mut partial = InterventionExposureLedger::default();
            partial
                .append(
                    fixture_event(
                        "BERTH-102-DIRECT-DISCHARGE",
                        "TREATMENT-VISIT-001",
                        InterventionStage::Announced,
                        0,
                        "aggregate-announcement-only",
                        None,
                    ),
                    1_900_000_000,
                )
                .expect("announcement only fixture");
            partial
        };
        let mut assessment = fixture_assessment();
        assessment.design = case.design;
        assessment.pretrend = case.pretrend;
        assessment.interference = case.spillover;
        assessment.outcomes = case.outcomes;
        if !case.exposure_complete {
            assessment.treatment_unit_ids = treatment_units
                .iter()
                .map(|unit| (*unit).to_owned())
                .collect();
        }
        let decision = assess_causal_identification(&ledger, &assessment);
        assert_eq!(decision.label(), case.expected, "causal gate case {}", case.id);
        let exposure_state = if case.exposure_complete {
            "REALIZED_UNIT_LEVEL"
        } else {
            "ANNOUNCED_ONLY"
        };
        let claim_ceiling = match &decision {
            CausalIdentificationDecision::EligibleForIndependentEstimator { claim_ceiling, .. }
            | CausalIdentificationDecision::NotIdentified { claim_ceiling, .. }
            | CausalIdentificationDecision::DescriptiveOnly { claim_ceiling } => *claim_ceiling,
        };
        let _ = writeln!(
            output,
            "{},{},{:?},{:?},{:?},{},{},{},{claim_ceiling}",
            case.id,
            case.design.label(),
            case.pretrend,
            case.spillover,
            case.outcomes,
            exposure_state,
            decision.label(),
            decision.blocker_labels()
        );
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn append_complete_chain(
        ledger: &mut InterventionExposureLedger,
        intervention_id: &str,
        unit_id: &str,
    ) {
        for (index, stage) in [
            InterventionStage::Announced,
            InterventionStage::Committed,
            InterventionStage::Available,
            InterventionStage::Activated,
            InterventionStage::Realized,
        ]
        .iter()
        .enumerate()
        {
            ledger
                .append(
                    fixture_event(
                        intervention_id,
                        unit_id,
                        *stage,
                        index as i64 * 100,
                        &format!("{unit_id}-{stage:?}"),
                        (*stage == InterventionStage::Realized).then_some(25_000),
                    ),
                    1_900_000_000,
                )
                .expect("valid chain");
        }
    }

    #[test]
    fn intervention_stages_must_be_recorded_in_order() {
        let mut ledger = InterventionExposureLedger::default();
        assert_eq!(
            ledger.append(
                fixture_event(
                    "BERTH-102",
                    "VISIT-1",
                    InterventionStage::Available,
                    0,
                    "available-first",
                    None
                ),
                1_900_000_000
            ),
            Err(ExposureLedgerError::FirstStageMustBeAnnounced)
        );

        ledger
            .append(
                fixture_event(
                    "BERTH-102",
                    "VISIT-1",
                    InterventionStage::Announced,
                    0,
                    "announced",
                    None
                ),
                1_900_000_000
            )
            .expect("announced");
        assert_eq!(
            ledger.append(
                fixture_event(
                    "BERTH-102",
                    "VISIT-1",
                    InterventionStage::Available,
                    100,
                    "skipped-commitment",
                    None
                ),
                1_900_000_000
            ),
            Err(ExposureLedgerError::StageTransitionSkippedOrReversed)
        );
    }

    #[test]
    fn realization_requires_positive_measured_output() {
        let mut ledger = InterventionExposureLedger::default();
        for (index, stage) in [
            InterventionStage::Announced,
            InterventionStage::Committed,
            InterventionStage::Available,
            InterventionStage::Activated,
        ]
        .iter()
        .enumerate()
        {
            ledger
                .append(
                    fixture_event("BERTH-102", "VISIT-1", *stage, index as i64, &format!("source-{index}"), None),
                    1_900_000_000,
                )
                .expect("valid stage");
        }
        assert_eq!(
            ledger.append(
                fixture_event(
                    "BERTH-102",
                    "VISIT-1",
                    InterventionStage::Realized,
                    10,
                    "realized-without-quantity",
                    None
                ),
                1_900_000_000
            ),
            Err(ExposureLedgerError::RealizedQuantityMissing)
        );
        assert_eq!(
            ledger.append(
                fixture_event(
                    "BERTH-102",
                    "VISIT-1",
                    InterventionStage::Realized,
                    10,
                    "realized-zero-quantity",
                    Some(0)
                ),
                1_900_000_000
            ),
            Err(ExposureLedgerError::RealizedQuantityMustBePositive)
        );
    }

    #[test]
    fn historical_asof_does_not_use_later_retrieved_activation() {
        let mut ledger = InterventionExposureLedger::default();
        append_complete_chain(&mut ledger, "BERTH-102", "VISIT-1");
        let current = ledger.current_exposure("BERTH-102", "VISIT-1", "BERTH-102", 1_900_000_000);
        assert_eq!(current.expect("current").stage, InterventionStage::Realized);
        assert!(!ledger.unit_is_realized("BERTH-102", "VISIT-1", 1_800_000_250));
    }

    #[test]
    fn randomized_assignment_requires_verified_randomization_design() {
        let ledger = fixture_ledger(&["TREATMENT-VISIT-001", "TREATMENT-VISIT-002"]);
        let mut assessment = fixture_assessment();
        assessment.design = CausalDesign::RandomizedAssignment;
        assessment.comparison = ComparisonEvidence::ConcurrentMatchedControl;
        assert!(matches!(
            assess_causal_identification(&ledger, &assessment),
            CausalIdentificationDecision::NotIdentified { blockers, .. }
                if blockers.contains(&CausalBlocker::RandomizationNotVerified)
        ));
    }

    #[test]
    fn did_requires_parallel_trend_overlap_confonder_and_spillover_reviews() {
        let ledger = fixture_ledger(&["TREATMENT-VISIT-001", "TREATMENT-VISIT-002"]);
        let mut assessment = fixture_assessment();
        assessment.pretrend = PreTrendEvidence::Failed;
        assessment.overlap = OverlapEvidence::NotAssessed;
        assessment.interference = InterferenceEvidence::UnmodelledSpillovers;
        assessment.confounding = ConfoundingEvidence::NotAssessed;
        assessment.co_interventions = CoInterventionEvidence::NotAssessed;
        let decision = assess_causal_identification(&ledger, &assessment);
        assert!(matches!(
            decision,
            CausalIdentificationDecision::NotIdentified { ref blockers, .. }
                if blockers.contains(&CausalBlocker::PreTrendFailed)
                    && blockers.contains(&CausalBlocker::OverlapNotAssessed)
                    && blockers.contains(&CausalBlocker::SpilloversUnmodelled)
                    && blockers.contains(&CausalBlocker::ConfoundingNotAssessed)
                    && blockers.contains(&CausalBlocker::CoInterventionsNotAssessed)
        ));
    }

    #[test]
    fn control_unit_activated_by_same_intervention_is_contaminated() {
        let mut ledger = fixture_ledger(&["TREATMENT-VISIT-001", "TREATMENT-VISIT-002"]);
        append_complete_chain(&mut ledger, "BERTH-102-DIRECT-DISCHARGE", "CONTROL-VISIT-001");
        let decision = assess_causal_identification(&ledger, &fixture_assessment());
        assert!(matches!(
            decision,
            CausalIdentificationDecision::NotIdentified { blockers, .. }
                if blockers.contains(&CausalBlocker::ComparisonUnitContaminated)
        ));
    }

    #[test]
    fn descriptive_before_after_never_promotes_to_causal_effect() {
        let ledger = fixture_ledger(&["TREATMENT-VISIT-001", "TREATMENT-VISIT-002"]);
        let mut assessment = fixture_assessment();
        assessment.design = CausalDesign::DescriptiveBeforeAfter;
        assert!(matches!(
            assess_causal_identification(&ledger, &assessment),
            CausalIdentificationDecision::DescriptiveOnly { .. }
        ));
    }

    #[test]
    fn a_complete_did_case_is_only_eligible_for_external_estimator() {
        let ledger = fixture_ledger(&["TREATMENT-VISIT-001", "TREATMENT-VISIT-002"]);
        assert!(matches!(
            assess_causal_identification(&ledger, &fixture_assessment()),
            CausalIdentificationDecision::EligibleForIndependentEstimator {
                treatment_units: 2,
                comparison_units: 2,
                ..
            }
        ));
    }

    #[test]
    fn missing_evidence_reference_blocks_claim_even_when_flags_pass() {
        let ledger = fixture_ledger(&["TREATMENT-VISIT-001", "TREATMENT-VISIT-002"]);
        let mut assessment = fixture_assessment();
        assessment.evidence.interference_review_ref.clear();
        assert!(matches!(
            assess_causal_identification(&ledger, &assessment),
            CausalIdentificationDecision::NotIdentified { blockers, .. }
                if blockers.contains(&CausalBlocker::MissingEvidenceReference)
        ));
    }

    #[test]
    fn treatment_and_control_unit_sets_must_be_disjoint() {
        let ledger = fixture_ledger(&["TREATMENT-VISIT-001", "TREATMENT-VISIT-002"]);
        let mut assessment = fixture_assessment();
        assessment.comparison_unit_ids[0] = assessment.treatment_unit_ids[0].clone();
        assert!(matches!(
            assess_causal_identification(&ledger, &assessment),
            CausalIdentificationDecision::NotIdentified { blockers, .. }
                if blockers.contains(&CausalBlocker::TreatmentComparisonUnitsOverlap)
        ));
    }

    #[test]
    fn source_records_must_be_unique_and_events_available_as_of_cutoff() {
        let mut ledger = InterventionExposureLedger::default();
        let mut event = fixture_event(
            "BERTH-102",
            "VISIT-1",
            InterventionStage::Announced,
            0,
            "source-1",
            None,
        );
        event.published_at_unix = 1_900_000_200;
        event.retrieved_at_unix = 1_900_000_210;
        assert_eq!(
            ledger.append(event, 1_900_000_000),
            Err(ExposureLedgerError::EvidenceNotAvailableAtAsOf)
        );
    }

    #[test]
    fn deterministic_receipt_is_byte_identical_and_semantically_bounded() {
        let first = render_csv();
        assert_eq!(first, render_csv());
        assert_eq!(first.lines().count(), 7);
        let width = first.lines().next().expect("header").split(',').count();
        assert_eq!(width, 9);
        for row in first.lines().skip(1) {
            assert_eq!(row.split(',').count(), width);
        }
        assert!(first.contains("ELIGIBLE_FOR_INDEPENDENT_ESTIMATOR"));
        assert!(first.contains("NOT_IDENTIFIED"));
        assert!(first.contains("DESCRIPTIVE_ONLY"));
        assert!(first.contains("SPILLOVERS_UNMODELLED"));
        assert!(first.contains("RECEIVER_OUTCOMES_INCOMPLETE"));
        assert!(!first.contains("CAUSAL_EFFECT_ESTIMATE"));
    }
}
