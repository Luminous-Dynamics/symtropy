// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fail-closed composition checks for CivBench decision-to-service references.
//!
//! This module deliberately validates references and non-equivalences only.
//! It does not duplicate the event evaluator, resource ledger, work-order
//! lifecycle, VOI policy, authority system, hydraulic model or outcome delta
//! calculator. Each of those remains owned by its existing component.

use std::fmt::Write as _;

/// Disposition recorded by the existing human-authority plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanDisposition {
    /// A named human authority approved the proposed action.
    Authorized,
    /// The recommendation was rejected.
    Rejected,
    /// The recommendation was deferred.
    Deferred,
}

/// Resource-feasibility result emitted by the existing resource ledger/workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceFeasibility {
    /// Required resources were reserved successfully.
    FeasibleAndReserved,
    /// No compatible crew or contractor capacity was available.
    RejectedNoCapacity,
    /// A required part was not available.
    RejectedMissingPart,
    /// The declared budget was insufficient.
    RejectedBudget,
    /// Energy, access or a work-window constraint prevented execution.
    RejectedEnergyOrAccess,
    /// No reservation or feasibility request was attempted.
    NotAttempted,
}

/// Highest work stage supported by the referenced workflow receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionProgress {
    /// No work was planned or started.
    NotStarted,
    /// A work item was planned after authorization and reservation.
    Planned,
    /// Work execution began.
    InProgress,
    /// A provider or operator claimed work completion.
    RepairClaimed,
    /// A distinct verifier accepted the completed work.
    IndependentlyVerified,
    /// Work was cancelled and its outstanding reservation was released.
    Cancelled,
}

/// Evidence class on the referenced paired service-outcome receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceEvidenceClass {
    /// Outcomes came from a declared simulation/model.
    Modelled,
    /// Outcomes came from direct qualified observations.
    Observed,
    /// Outcomes were reproducibly derived from qualified inputs.
    Derived,
}

/// Direction and evidence class of a precomputed service comparison.
///
/// The adapter does not calculate this value. It checks that the referenced
/// owner receipt exists and that the claim is consistent with the source's
/// evidence class. Cross-status arithmetic remains prohibited by the paired
/// outcome core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceEffect {
    /// No service-effect claim is admitted.
    NotClaimed,
    /// A modelled paired comparison reports improvement.
    ModelledImprovement,
    /// An observed paired comparison reports improvement.
    ObservedImprovement,
    /// A derived paired comparison reports improvement.
    DerivedImprovement,
    /// A modelled paired comparison reports no improvement.
    ModelledNoImprovement,
    /// An observed paired comparison reports no improvement.
    ObservedNoImprovement,
    /// A derived paired comparison reports no improvement.
    DerivedNoImprovement,
    /// A modelled paired comparison reports regression.
    ModelledRegression,
    /// An observed paired comparison reports regression.
    ObservedRegression,
    /// A derived paired comparison reports regression.
    DerivedRegression,
    /// The effect is indeterminate or its source outcome is not measured.
    Indeterminate,
}

/// Opaque owner references for one decision-to-service composition episode.
///
/// The strings are immutable record identifiers produced by their owners.
/// This type does not copy source truth into a second ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionClosureRecord {
    /// Stable identity for this composition episode.
    pub decision_episode_id: String,
    /// Paired experiment block from the experiment qualification core.
    pub paired_block_id: String,
    /// Event-evaluator receipt from the event-level scoring core.
    pub event_receipt_id: String,
    /// Optional evidence-acquisition/VOI receipt from Sol-Atlas.
    pub acquisition_receipt_id: Option<String>,
    /// Whether acquisition changed the admissible decision or selected action.
    pub decision_changed_after_acquisition: Option<bool>,
    /// Human disposition from the existing authority plane.
    pub human_disposition: HumanDisposition,
    /// Feasibility/reservation result from the existing resource ledger.
    pub resource_feasibility: ResourceFeasibility,
    /// Highest workflow stage supported by the source workflow.
    pub action_progress: ActionProgress,
    /// Resource reservation/settlement/release receipt, including a failed
    /// reservation attempt when the action was infeasible.
    pub resource_receipt_id: Option<String>,
    /// Inspection/dispatch/repair workflow receipt.
    pub workflow_receipt_id: Option<String>,
    /// Independent verification receipt. A provider repair claim is not this.
    pub verification_receipt_id: Option<String>,
    /// Single-arm service outcome receipt, if one is available.
    pub service_outcome_receipt_id: Option<String>,
    /// Paired control/treatment service outcome receipt, if one is available.
    pub paired_outcome_receipt_id: Option<String>,
    /// Evidence class declared by the source paired outcome receipt.
    pub service_evidence_class: Option<ServiceEvidenceClass>,
    /// Direction reported by the source paired outcome receipt.
    pub service_effect: ServiceEffect,
}

/// Stable validation failure for a composition reference or non-equivalence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionClosureError {
    /// A required stable identifier is empty or contains whitespace.
    InvalidReference(&'static str),
    /// A decision-change result is present without an acquisition receipt.
    DecisionChangeWithoutAcquisition,
    /// Rejected/deferred recommendations were followed by action.
    ActionAfterNonAuthorization,
    /// Work was started/planned without a successful resource reservation.
    ActionWithoutFeasibility,
    /// The resource ledger did not provide a reservation/rejection receipt.
    MissingResourceReceipt,
    /// The workflow owner did not provide the work lifecycle receipt.
    MissingWorkflowReceipt,
    /// Work was marked independently verified without an independent receipt.
    VerifiedWithoutVerification,
    /// A verification receipt exists but the work is not at verified stage.
    VerificationWithoutVerifiedStage,
    /// An action was attempted even though feasibility was not attempted or denied.
    ActionAfterFeasibilityRejection,
    /// A service effect was claimed without a single-arm outcome receipt.
    ServiceEffectWithoutOutcome,
    /// A service effect was claimed without a paired control/treatment receipt.
    ServiceEffectWithoutPairedOutcome,
    /// A numeric effect was claimed without its source evidence class.
    ServiceEffectWithoutEvidenceClass,
    /// The declared service effect's evidence class disagrees with its source.
    ServiceEffectEvidenceMismatch,
    /// An improvement was claimed before independent work verification.
    ImprovementWithoutVerifiedAction,
    /// A service-effect reference was supplied while the effect is unclaimed.
    ReferencesWithoutServiceClaim,
}

impl std::fmt::Display for DecisionClosureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidReference(field) => write!(f, "invalid or empty reference: {field}"),
            Self::DecisionChangeWithoutAcquisition => {
                write!(f, "decision-change result requires an acquisition receipt")
            }
            Self::ActionAfterNonAuthorization => {
                write!(
                    f,
                    "rejected/deferred recommendation cannot be followed by action"
                )
            }
            Self::ActionWithoutFeasibility => {
                write!(f, "work requires a feasible resource reservation")
            }
            Self::MissingResourceReceipt => {
                write!(f, "resource feasibility requires a resource-ledger receipt")
            }
            Self::MissingWorkflowReceipt => {
                write!(f, "work progress requires a workflow receipt")
            }
            Self::VerifiedWithoutVerification => {
                write!(
                    f,
                    "verified work requires an independent verification receipt"
                )
            }
            Self::VerificationWithoutVerifiedStage => {
                write!(
                    f,
                    "verification receipt requires independently-verified work stage"
                )
            }
            Self::ActionAfterFeasibilityRejection => {
                write!(
                    f,
                    "work cannot proceed after a rejected/not-attempted reservation"
                )
            }
            Self::ServiceEffectWithoutOutcome => {
                write!(f, "service-effect claim requires a service outcome receipt")
            }
            Self::ServiceEffectWithoutPairedOutcome => {
                write!(f, "service-effect claim requires a paired outcome receipt")
            }
            Self::ServiceEffectWithoutEvidenceClass => {
                write!(f, "service-effect claim requires an evidence class")
            }
            Self::ServiceEffectEvidenceMismatch => {
                write!(
                    f,
                    "service-effect claim disagrees with source evidence class"
                )
            }
            Self::ImprovementWithoutVerifiedAction => {
                write!(
                    f,
                    "service improvement requires an authorized, independently verified action"
                )
            }
            Self::ReferencesWithoutServiceClaim => {
                write!(
                    f,
                    "outcome/evidence references are present without a service-effect claim"
                )
            }
        }
    }
}

impl std::error::Error for DecisionClosureError {}

fn valid_reference(reference: &str) -> bool {
    !reference.is_empty()
        && reference.trim() == reference
        && !reference.chars().any(char::is_whitespace)
}

fn validate_optional_reference(
    reference: &Option<String>,
    field: &'static str,
) -> Result<(), DecisionClosureError> {
    if reference
        .as_deref()
        .is_some_and(|value| !valid_reference(value))
    {
        return Err(DecisionClosureError::InvalidReference(field));
    }
    Ok(())
}

fn effect_class(effect: ServiceEffect) -> Option<ServiceEvidenceClass> {
    match effect {
        ServiceEffect::ModelledImprovement
        | ServiceEffect::ModelledNoImprovement
        | ServiceEffect::ModelledRegression => Some(ServiceEvidenceClass::Modelled),
        ServiceEffect::ObservedImprovement
        | ServiceEffect::ObservedNoImprovement
        | ServiceEffect::ObservedRegression => Some(ServiceEvidenceClass::Observed),
        ServiceEffect::DerivedImprovement
        | ServiceEffect::DerivedNoImprovement
        | ServiceEffect::DerivedRegression => Some(ServiceEvidenceClass::Derived),
        ServiceEffect::NotClaimed | ServiceEffect::Indeterminate => None,
    }
}

fn is_improvement(effect: ServiceEffect) -> bool {
    matches!(
        effect,
        ServiceEffect::ModelledImprovement
            | ServiceEffect::ObservedImprovement
            | ServiceEffect::DerivedImprovement
    )
}

fn is_service_claim(effect: ServiceEffect) -> bool {
    !matches!(
        effect,
        ServiceEffect::NotClaimed | ServiceEffect::Indeterminate
    )
}

impl DecisionClosureRecord {
    /// Validate the cross-owner decision-to-service references and invariants.
    ///
    /// This validates linkage and non-equivalence only. It does not re-evaluate
    /// the event receipt, admission decision, resource ledger, authority,
    /// workflow, verifier, hydraulic trajectory or paired outcome deltas.
    pub fn validate(&self) -> Result<(), DecisionClosureError> {
        for (value, field) in [
            (self.decision_episode_id.as_str(), "decision_episode_id"),
            (self.paired_block_id.as_str(), "paired_block_id"),
            (self.event_receipt_id.as_str(), "event_receipt_id"),
        ] {
            if !valid_reference(value) {
                return Err(DecisionClosureError::InvalidReference(field));
            }
        }

        validate_optional_reference(&self.acquisition_receipt_id, "acquisition_receipt_id")?;
        validate_optional_reference(&self.resource_receipt_id, "resource_receipt_id")?;
        validate_optional_reference(&self.workflow_receipt_id, "workflow_receipt_id")?;
        validate_optional_reference(&self.verification_receipt_id, "verification_receipt_id")?;
        validate_optional_reference(
            &self.service_outcome_receipt_id,
            "service_outcome_receipt_id",
        )?;
        validate_optional_reference(&self.paired_outcome_receipt_id, "paired_outcome_receipt_id")?;

        if self.decision_changed_after_acquisition.is_some()
            && self.acquisition_receipt_id.is_none()
        {
            return Err(DecisionClosureError::DecisionChangeWithoutAcquisition);
        }

        if matches!(
            self.human_disposition,
            HumanDisposition::Rejected | HumanDisposition::Deferred
        ) && self.action_progress != ActionProgress::NotStarted
        {
            return Err(DecisionClosureError::ActionAfterNonAuthorization);
        }

        if self.action_progress != ActionProgress::NotStarted
            && self.human_disposition != HumanDisposition::Authorized
        {
            return Err(DecisionClosureError::ActionAfterNonAuthorization);
        }

        if self.action_progress != ActionProgress::NotStarted {
            if self.resource_feasibility != ResourceFeasibility::FeasibleAndReserved {
                return Err(DecisionClosureError::ActionAfterFeasibilityRejection);
            }
            if self.resource_receipt_id.is_none() {
                return Err(DecisionClosureError::MissingResourceReceipt);
            }
            if self.workflow_receipt_id.is_none() {
                return Err(DecisionClosureError::MissingWorkflowReceipt);
            }
        }

        if self.resource_feasibility != ResourceFeasibility::NotAttempted
            && self.resource_receipt_id.is_none()
        {
            return Err(DecisionClosureError::MissingResourceReceipt);
        }

        if self.action_progress == ActionProgress::Cancelled && self.workflow_receipt_id.is_none() {
            return Err(DecisionClosureError::MissingWorkflowReceipt);
        }

        if self.action_progress == ActionProgress::IndependentlyVerified
            && self.verification_receipt_id.is_none()
        {
            return Err(DecisionClosureError::VerifiedWithoutVerification);
        }

        if self.verification_receipt_id.is_some()
            && self.action_progress != ActionProgress::IndependentlyVerified
        {
            return Err(DecisionClosureError::VerificationWithoutVerifiedStage);
        }

        match self.resource_feasibility {
            ResourceFeasibility::NotAttempted
            | ResourceFeasibility::RejectedNoCapacity
            | ResourceFeasibility::RejectedMissingPart
            | ResourceFeasibility::RejectedBudget
            | ResourceFeasibility::RejectedEnergyOrAccess => {
                if self.action_progress != ActionProgress::NotStarted {
                    return Err(DecisionClosureError::ActionAfterFeasibilityRejection);
                }
            }
            ResourceFeasibility::FeasibleAndReserved => {}
        }

        if is_service_claim(self.service_effect) {
            if self.service_outcome_receipt_id.is_none() {
                return Err(DecisionClosureError::ServiceEffectWithoutOutcome);
            }
            if self.paired_outcome_receipt_id.is_none() {
                return Err(DecisionClosureError::ServiceEffectWithoutPairedOutcome);
            }
            let expected = effect_class(self.service_effect);
            let Some(actual) = self.service_evidence_class else {
                return Err(DecisionClosureError::ServiceEffectWithoutEvidenceClass);
            };
            if expected != Some(actual) {
                return Err(DecisionClosureError::ServiceEffectEvidenceMismatch);
            }
        } else if self.service_effect == ServiceEffect::NotClaimed
            && (self.service_outcome_receipt_id.is_some()
                || self.paired_outcome_receipt_id.is_some()
                || self.service_evidence_class.is_some())
        {
            return Err(DecisionClosureError::ReferencesWithoutServiceClaim);
        } else if self.service_effect == ServiceEffect::Indeterminate
            && self.service_evidence_class.is_some()
        {
            return Err(DecisionClosureError::ServiceEffectEvidenceMismatch);
        }

        if is_improvement(self.service_effect)
            && (self.human_disposition != HumanDisposition::Authorized
                || self.action_progress != ActionProgress::IndependentlyVerified
                || self.verification_receipt_id.is_none())
        {
            return Err(DecisionClosureError::ImprovementWithoutVerifiedAction);
        }

        Ok(())
    }

    /// Produce deterministic line-oriented reference output.
    ///
    /// This is not a signature, digest or proof of source truth.
    pub fn canonical_text(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(&mut out, "CIVBENCH_DECISION_CLOSURE_V0.1");
        let _ = writeln!(&mut out, "decision_episode_id={}", self.decision_episode_id);
        let _ = writeln!(&mut out, "paired_block_id={}", self.paired_block_id);
        let _ = writeln!(&mut out, "event_receipt_id={}", self.event_receipt_id);
        let _ = writeln!(
            &mut out,
            "acquisition_receipt_id={}",
            option_ref(&self.acquisition_receipt_id)
        );
        let _ = writeln!(
            &mut out,
            "decision_changed_after_acquisition={}",
            self.decision_changed_after_acquisition
                .map_or_else(|| "none".to_owned(), |value| value.to_string())
        );
        let _ = writeln!(
            &mut out,
            "human_disposition={}",
            disposition_name(self.human_disposition)
        );
        let _ = writeln!(
            &mut out,
            "resource_feasibility={}",
            feasibility_name(self.resource_feasibility)
        );
        let _ = writeln!(
            &mut out,
            "action_progress={}",
            action_progress_name(self.action_progress)
        );
        let _ = writeln!(
            &mut out,
            "resource_receipt_id={}",
            option_ref(&self.resource_receipt_id)
        );
        let _ = writeln!(
            &mut out,
            "workflow_receipt_id={}",
            option_ref(&self.workflow_receipt_id)
        );
        let _ = writeln!(
            &mut out,
            "verification_receipt_id={}",
            option_ref(&self.verification_receipt_id)
        );
        let _ = writeln!(
            &mut out,
            "service_outcome_receipt_id={}",
            option_ref(&self.service_outcome_receipt_id)
        );
        let _ = writeln!(
            &mut out,
            "paired_outcome_receipt_id={}",
            option_ref(&self.paired_outcome_receipt_id)
        );
        let _ = writeln!(
            &mut out,
            "service_evidence_class={}",
            self.service_evidence_class
                .map_or("none", evidence_class_name)
        );
        let _ = writeln!(
            &mut out,
            "service_effect={}",
            service_effect_name(self.service_effect)
        );
        out
    }
}

fn option_ref(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("none")
}

fn disposition_name(value: HumanDisposition) -> &'static str {
    match value {
        HumanDisposition::Authorized => "authorized",
        HumanDisposition::Rejected => "rejected",
        HumanDisposition::Deferred => "deferred",
    }
}

fn feasibility_name(value: ResourceFeasibility) -> &'static str {
    match value {
        ResourceFeasibility::FeasibleAndReserved => "feasible_and_reserved",
        ResourceFeasibility::RejectedNoCapacity => "rejected_no_capacity",
        ResourceFeasibility::RejectedMissingPart => "rejected_missing_part",
        ResourceFeasibility::RejectedBudget => "rejected_budget",
        ResourceFeasibility::RejectedEnergyOrAccess => "rejected_energy_or_access",
        ResourceFeasibility::NotAttempted => "not_attempted",
    }
}

fn action_progress_name(value: ActionProgress) -> &'static str {
    match value {
        ActionProgress::NotStarted => "not_started",
        ActionProgress::Planned => "planned",
        ActionProgress::InProgress => "in_progress",
        ActionProgress::RepairClaimed => "repair_claimed",
        ActionProgress::IndependentlyVerified => "independently_verified",
        ActionProgress::Cancelled => "cancelled",
    }
}

fn evidence_class_name(value: ServiceEvidenceClass) -> &'static str {
    match value {
        ServiceEvidenceClass::Modelled => "modelled",
        ServiceEvidenceClass::Observed => "observed",
        ServiceEvidenceClass::Derived => "derived",
    }
}

fn service_effect_name(value: ServiceEffect) -> &'static str {
    match value {
        ServiceEffect::NotClaimed => "not_claimed",
        ServiceEffect::ModelledImprovement => "modelled_improvement",
        ServiceEffect::ObservedImprovement => "observed_improvement",
        ServiceEffect::DerivedImprovement => "derived_improvement",
        ServiceEffect::ModelledNoImprovement => "modelled_no_improvement",
        ServiceEffect::ObservedNoImprovement => "observed_no_improvement",
        ServiceEffect::DerivedNoImprovement => "derived_no_improvement",
        ServiceEffect::ModelledRegression => "modelled_regression",
        ServiceEffect::ObservedRegression => "observed_regression",
        ServiceEffect::DerivedRegression => "derived_regression",
        ServiceEffect::Indeterminate => "indeterminate",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn positive_record() -> DecisionClosureRecord {
        DecisionClosureRecord {
            decision_episode_id: "decision-1".into(),
            paired_block_id: "block-1".into(),
            event_receipt_id: "event-receipt-1".into(),
            acquisition_receipt_id: Some("acquisition-1".into()),
            decision_changed_after_acquisition: Some(true),
            human_disposition: HumanDisposition::Authorized,
            resource_feasibility: ResourceFeasibility::FeasibleAndReserved,
            action_progress: ActionProgress::IndependentlyVerified,
            resource_receipt_id: Some("resource-1".into()),
            workflow_receipt_id: Some("workflow-1".into()),
            verification_receipt_id: Some("verification-1".into()),
            service_outcome_receipt_id: Some("service-outcome-1".into()),
            paired_outcome_receipt_id: Some("paired-outcome-1".into()),
            service_evidence_class: Some(ServiceEvidenceClass::Modelled),
            service_effect: ServiceEffect::ModelledImprovement,
        }
    }

    #[test]
    fn independently_verified_modelled_improvement_is_valid() {
        let record = positive_record();
        assert_eq!(record.validate(), Ok(()));
        assert!(
            record
                .canonical_text()
                .contains("service_effect=modelled_improvement")
        );
    }

    #[test]
    fn acquisition_cannot_change_decision_without_an_acquisition_receipt() {
        let mut record = positive_record();
        record.acquisition_receipt_id = None;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::DecisionChangeWithoutAcquisition)
        );
    }

    #[test]
    fn rejected_recommendation_cannot_become_a_work_order() {
        let mut record = positive_record();
        record.human_disposition = HumanDisposition::Rejected;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ActionAfterNonAuthorization)
        );
    }

    #[test]
    fn deferred_recommendation_cannot_claim_a_repair() {
        let mut record = positive_record();
        record.human_disposition = HumanDisposition::Deferred;
        record.action_progress = ActionProgress::RepairClaimed;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ActionAfterNonAuthorization)
        );
    }

    #[test]
    fn authorized_but_infeasible_action_cannot_be_planned() {
        let mut record = positive_record();
        record.resource_feasibility = ResourceFeasibility::RejectedMissingPart;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ActionAfterFeasibilityRejection)
        );
    }

    #[test]
    fn verified_work_requires_a_distinct_verification_receipt() {
        let mut record = positive_record();
        record.verification_receipt_id = None;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::VerifiedWithoutVerification)
        );
    }

    #[test]
    fn verification_receipt_cannot_upgrade_a_repair_claim() {
        let mut record = positive_record();
        record.action_progress = ActionProgress::RepairClaimed;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::VerificationWithoutVerifiedStage)
        );
    }

    #[test]
    fn improvement_requires_authorization_feasibility_and_verification() {
        let mut record = positive_record();
        record.action_progress = ActionProgress::RepairClaimed;
        record.verification_receipt_id = None;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ImprovementWithoutVerifiedAction)
        );
    }

    #[test]
    fn a_service_claim_requires_both_single_and_paired_outcome_receipts() {
        let mut record = positive_record();
        record.paired_outcome_receipt_id = None;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ServiceEffectWithoutPairedOutcome)
        );
    }

    #[test]
    fn source_evidence_class_must_match_the_effect_claim() {
        let mut record = positive_record();
        record.service_evidence_class = Some(ServiceEvidenceClass::Observed);
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ServiceEffectEvidenceMismatch)
        );
    }

    #[test]
    fn no_value_sensor_can_resolve_without_becoming_a_service_gain() {
        let record = DecisionClosureRecord {
            decision_episode_id: "decision-no-value".into(),
            paired_block_id: "block-no-value".into(),
            event_receipt_id: "event-receipt-no-value".into(),
            acquisition_receipt_id: Some("acquisition-no-value".into()),
            decision_changed_after_acquisition: Some(false),
            human_disposition: HumanDisposition::Deferred,
            resource_feasibility: ResourceFeasibility::NotAttempted,
            action_progress: ActionProgress::NotStarted,
            resource_receipt_id: None,
            workflow_receipt_id: None,
            verification_receipt_id: None,
            service_outcome_receipt_id: Some("service-no-value".into()),
            paired_outcome_receipt_id: Some("paired-no-value".into()),
            service_evidence_class: Some(ServiceEvidenceClass::Modelled),
            service_effect: ServiceEffect::ModelledNoImprovement,
        };
        assert_eq!(record.validate(), Ok(()));
    }

    #[test]
    fn resource_exhaustion_is_auditable_but_not_executable() {
        let record = DecisionClosureRecord {
            decision_episode_id: "decision-no-part".into(),
            paired_block_id: "block-no-part".into(),
            event_receipt_id: "event-receipt-no-part".into(),
            acquisition_receipt_id: None,
            decision_changed_after_acquisition: None,
            human_disposition: HumanDisposition::Authorized,
            resource_feasibility: ResourceFeasibility::RejectedMissingPart,
            action_progress: ActionProgress::NotStarted,
            resource_receipt_id: Some("resource-rejection-no-part".into()),
            workflow_receipt_id: None,
            verification_receipt_id: None,
            service_outcome_receipt_id: None,
            paired_outcome_receipt_id: None,
            service_evidence_class: None,
            service_effect: ServiceEffect::Indeterminate,
        };
        assert_eq!(record.validate(), Ok(()));
    }

    #[test]
    fn unverified_repair_remains_indeterminate() {
        let mut record = positive_record();
        record.action_progress = ActionProgress::RepairClaimed;
        record.verification_receipt_id = None;
        record.service_outcome_receipt_id = None;
        record.paired_outcome_receipt_id = None;
        record.service_evidence_class = None;
        record.service_effect = ServiceEffect::Indeterminate;
        assert_eq!(record.validate(), Ok(()));
    }

    #[test]
    fn unclaimed_service_effect_cannot_carry_effect_receipts() {
        let mut record = positive_record();
        record.service_effect = ServiceEffect::NotClaimed;
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::ReferencesWithoutServiceClaim)
        );
    }

    #[test]
    fn whitespace_and_empty_references_fail_closed() {
        let mut record = positive_record();
        record.paired_block_id = "block with spaces".into();
        assert_eq!(
            record.validate(),
            Err(DecisionClosureError::InvalidReference("paired_block_id"))
        );
    }

    #[test]
    fn canonical_output_is_deterministic_for_identical_references() {
        let first = positive_record();
        let second = positive_record();
        assert_eq!(first.canonical_text(), second.canonical_text());
    }
}
