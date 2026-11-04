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

/// Owner expected to issue one reference in the decision-closure chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReceiptOwner {
    /// Event-level alarm/incident evaluator.
    EventEvaluator,
    /// Evidence acquisition / value-of-information planner.
    AcquisitionPlanner,
    /// Human authority and disposition plane.
    HumanAuthority,
    /// Finite resource ledger.
    ResourceLedger,
    /// Work-order / repair workflow.
    RepairWorkflow,
    /// Independent verification system.
    IndependentVerifier,
    /// Single-arm service outcome source.
    ServiceOutcome,
    /// Paired control/treatment outcome evaluator.
    PairedOutcome,
}

/// Owner-issued receipt identifier bound to this decision episode and paired block.
///
/// The adapter checks the declared binding and digest shape. It does not fetch
/// receipt bytes, verify signatures, or authenticate the issuer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptBinding {
    /// Owner role for this receipt.
    pub owner: ReceiptOwner,
    /// Immutable source-issued identifier.
    pub receipt_id: String,
    /// Decision episode to which the receipt is bound.
    pub decision_episode_id: String,
    /// Matched experiment block to which the receipt is bound.
    pub paired_block_id: String,
    /// Declared SHA-256 digest of the receipt payload.
    pub payload_digest: String,
}

/// Resource reservation lifecycle state reported by the existing ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceSettlement {
    /// No reservation was attempted.
    NotAttempted,
    /// Reservation was rejected; no resources were held.
    Rejected,
    /// Resources remain reserved for planned/active work.
    Reserved,
    /// Completed work settled the reservation.
    Settled,
    /// Cancelled work released its reservation.
    Released,
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
    /// Owner-issued authority receipt for the recorded disposition.
    pub human_authority_receipt_id: Option<String>,
    /// Opaque principal reference for the human decision-maker.
    pub human_actor_ref: Option<String>,
    /// Feasibility/reservation result from the existing resource ledger.
    pub resource_feasibility: ResourceFeasibility,
    /// Reservation/settlement state from the resource ledger.
    pub resource_settlement: ResourceSettlement,
    /// Principal reference for the person/team executing the work.
    pub executor_identity_ref: Option<String>,
    /// Principal reference for the independent verifier.
    pub verifier_identity_ref: Option<String>,
    /// Owner-typed bindings for every receipt referenced in this record.
    pub receipt_bindings: Vec<ReceiptBinding>,
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
    /// A human disposition lacks its owner-issued authority receipt.
    MissingAuthorityReceipt,
    /// A referenced receipt lacks an owner/context/digest binding.
    MissingReceiptBinding,
    /// A receipt binding does not match its reference or decision context.
    ReceiptBindingMismatch,
    /// A receipt ID is reused across owner roles.
    DuplicateReceiptId,
    /// A receipt payload digest is malformed or the zero sentinel.
    InvalidReceiptDigest,
    /// Work progress lacks an executor principal reference.
    MissingExecutorIdentity,
    /// Independent verification lacks a verifier principal reference.
    MissingVerifierIdentity,
    /// The verifier principal is the same as the executor principal.
    VerifierNotIndependent,
    /// Resource settlement is inconsistent with work progress.
    ResourceSettlementMismatch,
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
            Self::MissingAuthorityReceipt => {
                write!(f, "human disposition requires an authority receipt")
            }
            Self::MissingReceiptBinding => {
                write!(
                    f,
                    "a referenced receipt is missing its owner/context binding"
                )
            }
            Self::ReceiptBindingMismatch => {
                write!(
                    f,
                    "receipt binding does not match its reference or decision context"
                )
            }
            Self::DuplicateReceiptId => {
                write!(f, "receipt ID is reused across distinct owner roles")
            }
            Self::InvalidReceiptDigest => {
                write!(
                    f,
                    "receipt payload digest must be non-zero SHA-256 hexadecimal"
                )
            }
            Self::MissingExecutorIdentity => {
                write!(f, "work progress requires an executor identity reference")
            }
            Self::MissingVerifierIdentity => {
                write!(
                    f,
                    "independent verification requires a verifier identity reference"
                )
            }
            Self::VerifierNotIndependent => {
                write!(f, "verifier identity must differ from executor identity")
            }
            Self::ResourceSettlementMismatch => {
                write!(f, "resource settlement does not match work progress")
            }
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

fn valid_sha256(digest: &str) -> bool {
    digest.len() == 64
        && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        && digest.bytes().any(|byte| byte != b'0')
}

fn expected_receipt_bindings(record: &DecisionClosureRecord) -> Vec<(ReceiptOwner, &str)> {
    let mut expected = vec![(
        ReceiptOwner::EventEvaluator,
        record.event_receipt_id.as_str(),
    )];
    if let Some(id) = record.acquisition_receipt_id.as_deref() {
        expected.push((ReceiptOwner::AcquisitionPlanner, id));
    }
    if let Some(id) = record.human_authority_receipt_id.as_deref() {
        expected.push((ReceiptOwner::HumanAuthority, id));
    }
    if let Some(id) = record.resource_receipt_id.as_deref() {
        expected.push((ReceiptOwner::ResourceLedger, id));
    }
    if let Some(id) = record.workflow_receipt_id.as_deref() {
        expected.push((ReceiptOwner::RepairWorkflow, id));
    }
    if let Some(id) = record.verification_receipt_id.as_deref() {
        expected.push((ReceiptOwner::IndependentVerifier, id));
    }
    if let Some(id) = record.service_outcome_receipt_id.as_deref() {
        expected.push((ReceiptOwner::ServiceOutcome, id));
    }
    if let Some(id) = record.paired_outcome_receipt_id.as_deref() {
        expected.push((ReceiptOwner::PairedOutcome, id));
    }
    expected
}

fn validate_receipt_bindings(record: &DecisionClosureRecord) -> Result<(), DecisionClosureError> {
    let expected = expected_receipt_bindings(record);
    if record.receipt_bindings.len() != expected.len() {
        return Err(DecisionClosureError::MissingReceiptBinding);
    }

    let mut seen_ids = std::collections::BTreeSet::new();
    let mut seen_owners = std::collections::BTreeSet::new();
    for binding in &record.receipt_bindings {
        if !valid_reference(&binding.receipt_id)
            || !valid_reference(&binding.decision_episode_id)
            || !valid_reference(&binding.paired_block_id)
        {
            return Err(DecisionClosureError::InvalidReference("receipt_binding"));
        }
        if !valid_sha256(&binding.payload_digest) {
            return Err(DecisionClosureError::InvalidReceiptDigest);
        }
        if binding.decision_episode_id != record.decision_episode_id
            || binding.paired_block_id != record.paired_block_id
        {
            return Err(DecisionClosureError::ReceiptBindingMismatch);
        }
        if !seen_ids.insert(binding.receipt_id.as_str()) || !seen_owners.insert(binding.owner) {
            return Err(DecisionClosureError::DuplicateReceiptId);
        }
        if !expected
            .iter()
            .any(|(owner, id)| *owner == binding.owner && *id == binding.receipt_id)
        {
            return Err(DecisionClosureError::ReceiptBindingMismatch);
        }
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

        if self.human_authority_receipt_id.is_none() {
            return Err(DecisionClosureError::MissingAuthorityReceipt);
        }
        if self.human_actor_ref.is_none() {
            return Err(DecisionClosureError::InvalidReference("human_actor_ref"));
        }

        validate_optional_reference(
            &self.human_authority_receipt_id,
            "human_authority_receipt_id",
        )?;
        validate_optional_reference(&self.human_actor_ref, "human_actor_ref")?;
        validate_optional_reference(&self.executor_identity_ref, "executor_identity_ref")?;
        validate_optional_reference(&self.verifier_identity_ref, "verifier_identity_ref")?;
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
            if self.executor_identity_ref.is_none() {
                return Err(DecisionClosureError::MissingExecutorIdentity);
            }
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

        if self.action_progress == ActionProgress::IndependentlyVerified {
            let Some(verifier) = self.verifier_identity_ref.as_deref() else {
                return Err(DecisionClosureError::MissingVerifierIdentity);
            };
            if self.executor_identity_ref.as_deref() == Some(verifier) {
                return Err(DecisionClosureError::VerifierNotIndependent);
            }
        } else if self.verifier_identity_ref.is_some() {
            return Err(DecisionClosureError::VerificationWithoutVerifiedStage);
        }

        match (
            self.action_progress,
            self.resource_feasibility,
            self.resource_settlement,
        ) {
            (
                ActionProgress::NotStarted,
                ResourceFeasibility::NotAttempted,
                ResourceSettlement::NotAttempted,
            ) => {}
            (
                ActionProgress::NotStarted,
                ResourceFeasibility::RejectedNoCapacity
                | ResourceFeasibility::RejectedMissingPart
                | ResourceFeasibility::RejectedBudget
                | ResourceFeasibility::RejectedEnergyOrAccess,
                ResourceSettlement::Rejected,
            ) => {}
            (
                ActionProgress::NotStarted | ActionProgress::Planned | ActionProgress::InProgress,
                ResourceFeasibility::FeasibleAndReserved,
                ResourceSettlement::Reserved,
            ) => {}
            (
                ActionProgress::RepairClaimed | ActionProgress::IndependentlyVerified,
                ResourceFeasibility::FeasibleAndReserved,
                ResourceSettlement::Settled,
            ) => {}
            (
                ActionProgress::Cancelled,
                ResourceFeasibility::FeasibleAndReserved,
                ResourceSettlement::Released,
            ) => {}
            _ => return Err(DecisionClosureError::ResourceSettlementMismatch),
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

        validate_receipt_bindings(self)?;
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
            "human_authority_receipt_id={}",
            option_ref(&self.human_authority_receipt_id)
        );
        let _ = writeln!(
            &mut out,
            "human_actor_ref={}",
            option_ref(&self.human_actor_ref)
        );
        let _ = writeln!(
            &mut out,
            "resource_feasibility={}",
            feasibility_name(self.resource_feasibility)
        );
        let _ = writeln!(
            &mut out,
            "resource_settlement={}",
            settlement_name(self.resource_settlement)
        );
        let _ = writeln!(
            &mut out,
            "executor_identity_ref={}",
            option_ref(&self.executor_identity_ref)
        );
        let _ = writeln!(
            &mut out,
            "verifier_identity_ref={}",
            option_ref(&self.verifier_identity_ref)
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
        let mut bindings = self.receipt_bindings.clone();
        bindings.sort_by(|left, right| {
            left.owner
                .cmp(&right.owner)
                .then_with(|| left.receipt_id.cmp(&right.receipt_id))
        });
        for binding in &bindings {
            let _ = writeln!(
                &mut out,
                "receipt_binding={:?},{},{},{},{}",
                binding.owner,
                binding.receipt_id,
                binding.decision_episode_id,
                binding.paired_block_id,
                binding.payload_digest
            );
        }
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

fn settlement_name(value: ResourceSettlement) -> &'static str {
    match value {
        ResourceSettlement::NotAttempted => "not_attempted",
        ResourceSettlement::Rejected => "rejected",
        ResourceSettlement::Reserved => "reserved",
        ResourceSettlement::Settled => "settled",
        ResourceSettlement::Released => "released",
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
