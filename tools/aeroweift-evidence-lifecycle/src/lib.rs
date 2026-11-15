//! Finite composition oracle for the AeroWeft model-evidence lifecycle.
//!
//! This crate checks transition ordering and evidence linkage only. Owner-supplied
//! trust, qualification, authorization and adjudication states are inputs, not
//! independently authenticated facts. A passing lifecycle never grants flight,
//! dispatch, separation, execution or regulatory authority.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

/// Lifecycle state carried by one claim assessment.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LifecycleState {
    /// No prior model qualification exists.
    Uninitialized,
    /// An exact model claim is qualified for its declared scope.
    QualifiedCurrent,
    /// A historical claim is preserved but no longer relied upon.
    HistoricalOnly,
    /// A known material change requires targeted requalification.
    RequalificationRequired,
    /// Change-feed, provenance or evidence coverage is incomplete.
    Indeterminate,
    /// A qualified corrective event contradicts an earlier premise.
    Contradicted,
    /// A Track V candidate is selected; this is not authorization.
    TestDesignReady,
    /// No candidate was admissible.
    AbstainNoAdmissibleTest,
    /// External authority denied the exact test design.
    AuthorizationDenied,
    /// External authority approved this exact test design only.
    AuthorizedForDeclaredTestOnly,
    /// An experiment is in progress.
    ExecutionPending,
    /// An experiment was executed but not independently adjudicated.
    ExecutedAwaitingAdjudication,
    /// The experiment result was independently adjudicated for the declared profile.
    ResultAdjudicated,
    /// The experiment result was rejected by the adjudication owner.
    ResultRejected,
    /// The experiment result is indeterminate.
    ResultIndeterminate,
    /// A fresh Track T assessment accepted the named requalification claims.
    RequalificationAccepted,
}

/// Evidence-owner stage that emitted a transition or anchor receipt.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ReceiptStage {
    /// Track P subject/causal graph identity anchor.
    SubjectBinding,
    /// Track Q trust-policy and provenance disposition anchor.
    TrustAssessment,
    /// Track T model qualification or requalification.
    ModelQualification,
    /// Track U change-impact and applicability assessment.
    ChangeAssessment,
    /// Track V candidate experiment-selection assessment.
    ExperimentSelection,
    /// External operator/test authority decision for one exact design.
    ExternalAuthorization,
    /// Evidence that the exact selected experiment was executed.
    TestExecution,
    /// Independent adjudication of the execution result and reference data.
    ResultAdjudication,
}

/// Owner-supplied trust disposition. Cryptographic verification is outside this crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustDisposition {
    /// Trust owner says required facets meet the declared relying-party policy.
    QualifiedForDeclaredPolicy,
    /// Trust owner reports an indeterminate or incomplete result.
    Indeterminate,
    /// Trust owner says the evidence is not authorized or is rejected.
    Rejected,
}

/// Owner-supplied result disposition from Track U.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeDisposition {
    /// No relevant material change under complete declared coverage.
    StillApplicable,
    /// Historical evidence is retained but not current for reliance.
    HistoricalOnly,
    /// Known material change affects one or more claim dimensions.
    RequalificationRequired,
    /// A corrective event contradicts an earlier premise.
    Contradicted,
    /// Required event coverage or authority evidence is incomplete.
    Indeterminate,
}

/// Owner-supplied experiment-selection disposition from Track V.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionDisposition {
    /// A candidate passed the declared admission policy and was selected.
    CandidateSelected,
    /// No candidate was admissible; selection must stop.
    Abstain,
}

/// Owner-supplied external test-authority decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorizationDisposition {
    /// Exact design authorization was granted for the declared test only.
    Authorized,
    /// Authorization was explicitly denied.
    Denied,
}

/// Owner-supplied result-adjudication decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdjudicationDisposition {
    /// Independent adjudicator accepted the result under the declared profile.
    Accepted,
    /// Independent adjudicator rejected the result.
    Rejected,
    /// Result cannot be assessed under the declared profile.
    Indeterminate,
}

/// Exact parent receipt reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParentRef {
    /// Parent receipt ID.
    pub receipt_id: String,
    /// Exact parent payload digest copied from the referenced receipt.
    pub payload_sha256: String,
}

/// One immutable lifecycle transition or evidence anchor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecycleReceipt {
    /// Unique receipt identity within this supplied graph.
    pub receipt_id: String,
    /// Claim being advanced or evidenced.
    pub claim_id: String,
    /// Owner stage that emitted this receipt.
    pub stage: ReceiptStage,
    /// Exact aircraft/operation/flight subject digest, unchanged through this lifecycle.
    pub subject_scope_sha256: String,
    /// Exact model/configuration scope associated with this receipt.
    pub model_scope_sha256: String,
    /// Receipt payload digest string; bytes are not authenticated here.
    pub payload_sha256: String,
    /// Exact consumed parent receipts.
    pub parents: Vec<ParentRef>,
    /// Prior state for transition receipts; absent for evidence anchors.
    pub from_state: Option<LifecycleState>,
    /// Successor state for transition receipts; absent for evidence anchors.
    pub to_state: Option<LifecycleState>,
    /// Trust disposition, populated only by a TrustAssessment anchor.
    pub trust_disposition: Option<TrustDisposition>,
    /// Change disposition, populated only by a ChangeAssessment.
    pub change_disposition: Option<ChangeDisposition>,
    /// Selection disposition, populated only by an ExperimentSelection.
    pub selection_disposition: Option<SelectionDisposition>,
    /// External authorization disposition, populated only by authorization receipts.
    pub authorization_disposition: Option<AuthorizationDisposition>,
    /// Whether this selected candidate requires external authorization before execution.
    pub authorization_required: Option<bool>,
    /// Exact selected Track V candidate ID.
    pub candidate_id: Option<String>,
    /// Exact test design digest.
    pub design_sha256: Option<String>,
    /// Exact design digest covered by the external authorization decision.
    pub authorization_scope_sha256: Option<String>,
    /// Unique execution attempt identity.
    pub test_attempt_id: Option<String>,
    /// Digest of the observed execution-result manifest.
    pub result_manifest_sha256: Option<String>,
    /// Digest of independent reference/calibration evidence used in adjudication.
    pub independent_reference_sha256: Option<String>,
    /// Digest of the fresh Track T confirmatory dataset manifest.
    pub confirmatory_manifest_sha256: Option<String>,
    /// Digest of the original qualification's confirmatory manifest.
    pub prior_confirmatory_manifest_sha256: Option<String>,
    /// Owner-supplied result-adjudication disposition.
    pub adjudication_disposition: Option<AdjudicationDisposition>,
}

/// Frozen lifecycle policy independent of the submitted receipt graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecyclePolicy {
    /// Exact claim being followed.
    pub claim_id: String,
    /// Exact aircraft/operation/flight subject scope.
    pub subject_scope_sha256: String,
    /// Exact model scope of the original Track T qualification.
    pub initial_model_scope_sha256: String,
    /// Exact target model scope for requalification after the declared change.
    pub target_model_scope_sha256: String,
    /// Exact original Track T confirmatory manifest.
    pub prior_confirmatory_manifest_sha256: String,
}

/// Final deterministic lifecycle replay result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecycleResult {
    /// Terminal lifecycle state at the declared root.
    pub terminal_state: LifecycleState,
    /// Exact canonical receipt IDs included in the rooted evidence graph.
    pub receipt_ids: Vec<String>,
    /// Exact selected candidate, if a candidate was selected.
    pub candidate_id: Option<String>,
    /// Exact test attempt, if execution occurred.
    pub test_attempt_id: Option<String>,
    /// Exact new Track T confirmatory manifest, if requalification was accepted.
    pub accepted_confirmatory_manifest_sha256: Option<String>,
}

/// Typed structural rejection of a lifecycle graph.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LifecycleError {
    /// Frozen policy or one of its exact digests is malformed.
    InvalidPolicy,
    /// A receipt has malformed IDs, digest fields, stage or transition shape.
    InvalidReceipt,
    /// Receipt claim or exact subject identity does not match the frozen policy.
    SubjectOrClaimMismatch,
    /// One receipt ID occurs more than once.
    DuplicateReceiptId,
    /// A parent receipt is absent.
    ParentMissing,
    /// A parent digest does not match the exact payload digest of the referenced receipt.
    ParentDigestMismatch,
    /// Parent role, stage or transition does not match the lifecycle contract.
    StageOrTransitionMismatch,
    /// A required prior evidence receipt is absent.
    RequiredParentMissing,
    /// A duplicate parent edge was supplied.
    DuplicateParentEdge,
    /// The receipt graph contains a directed cycle.
    Cycle,
    /// One or more supplied receipts are not reachable from the declared root.
    OrphanReceipt,
    /// A Track U result is not eligible to proceed to experiment selection.
    ChangeDispositionNotActionable,
    /// A Track V result selected no candidate or failed to bind its design.
    CandidateNotSelected,
    /// External authorization is missing, denied or bound to another design.
    AuthorizationNotGrantedForDesign,
    /// Execution receipt does not match the selected candidate/design/authorization.
    ExecutionDoesNotMatchSelection,
    /// Result has not been independently adjudicated.
    ResultNotAdjudicated,
    /// Result is rejected or indeterminate.
    ResultNotAccepted,
    /// The new Track T qualification does not bind a fresh confirmatory manifest.
    ConfirmatoryManifestNotFresh,
    /// New Track T qualification does not match the exact target model scope.
    TargetModelScopeMismatch,
    /// A proposed test or execution receipt was incorrectly treated as requalification.
    EvidenceStageCannotQualifyModel,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
}

fn has_parent_stage(
    receipt: &LifecycleReceipt,
    stage: ReceiptStage,
    by_id: &BTreeMap<&str, &LifecycleReceipt>,
) -> bool {
    receipt.parents.iter().any(|parent| {
        by_id
            .get(parent.receipt_id.as_str())
            .is_some_and(|item| item.stage == stage)
    })
}

fn require_parent_stage(
    receipt: &LifecycleReceipt,
    stage: ReceiptStage,
    by_id: &BTreeMap<&str, &LifecycleReceipt>,
) -> Result<(), LifecycleError> {
    if has_parent_stage(receipt, stage, by_id) {
        Ok(())
    } else {
        Err(LifecycleError::RequiredParentMissing)
    }
}

fn validate_receipt_shape(receipt: &LifecycleReceipt) -> Result<(), LifecycleError> {
    match receipt.stage {
        ReceiptStage::SubjectBinding => {
            if receipt.from_state.is_some()
                || receipt.to_state.is_some()
                || receipt.trust_disposition.is_some()
                || receipt.change_disposition.is_some()
                || receipt.selection_disposition.is_some()
                || receipt.authorization_disposition.is_some()
                || receipt.authorization_required.is_some()
                || !receipt.parents.is_empty()
            {
                return Err(LifecycleError::InvalidReceipt);
            }
        }
        ReceiptStage::TrustAssessment => {
            if receipt.from_state.is_some()
                || receipt.to_state.is_some()
                || receipt.trust_disposition.is_none()
                || receipt.change_disposition.is_some()
                || receipt.selection_disposition.is_some()
                || receipt.authorization_disposition.is_some()
                || receipt.authorization_required.is_some()
            {
                return Err(LifecycleError::InvalidReceipt);
            }
        }
        _ => {
            if receipt.from_state.is_none() || receipt.to_state.is_none() {
                return Err(LifecycleError::InvalidReceipt);
            }
        }
    }

    match receipt.stage {
        ReceiptStage::ModelQualification => {
            if receipt.from_state == Some(LifecycleState::Uninitialized)
                && receipt.to_state == Some(LifecycleState::QualifiedCurrent)
            {
                if !is_sha256(
                    receipt
                        .confirmatory_manifest_sha256
                        .as_deref()
                        .unwrap_or_default(),
                ) {
                    return Err(LifecycleError::InvalidReceipt);
                }
            } else if receipt.from_state == Some(LifecycleState::ResultAdjudicated)
                && receipt.to_state == Some(LifecycleState::RequalificationAccepted)
            {
                if !is_sha256(
                    receipt
                        .confirmatory_manifest_sha256
                        .as_deref()
                        .unwrap_or_default(),
                ) || !is_sha256(
                    receipt
                        .prior_confirmatory_manifest_sha256
                        .as_deref()
                        .unwrap_or_default(),
                ) || receipt.confirmatory_manifest_sha256
                    == receipt.prior_confirmatory_manifest_sha256
                {
                    return Err(LifecycleError::ConfirmatoryManifestNotFresh);
                }
            } else {
                return Err(LifecycleError::EvidenceStageCannotQualifyModel);
            }
        }
        ReceiptStage::ChangeAssessment => {
            if receipt.from_state != Some(LifecycleState::QualifiedCurrent)
                || receipt.to_state.is_none()
                || receipt.change_disposition.is_none()
            {
                return Err(LifecycleError::StageOrTransitionMismatch);
            }
            let expected = match receipt.to_state {
                Some(LifecycleState::HistoricalOnly) => ChangeDisposition::HistoricalOnly,
                Some(LifecycleState::RequalificationRequired) => {
                    ChangeDisposition::RequalificationRequired
                }
                Some(LifecycleState::Contradicted) => ChangeDisposition::Contradicted,
                Some(LifecycleState::Indeterminate) => ChangeDisposition::Indeterminate,
                Some(LifecycleState::QualifiedCurrent) => ChangeDisposition::StillApplicable,
                _ => return Err(LifecycleError::StageOrTransitionMismatch),
            };
            if receipt.change_disposition != Some(expected) {
                return Err(LifecycleError::StageOrTransitionMismatch);
            }
        }
        ReceiptStage::ExperimentSelection => {
            if receipt.from_state != Some(LifecycleState::RequalificationRequired)
                || !matches!(
                    receipt.to_state,
                    Some(
                        LifecycleState::TestDesignReady
                            | LifecycleState::AbstainNoAdmissibleTest
                    )
                )
                || receipt.selection_disposition.is_none()
            {
                return Err(LifecycleError::StageOrTransitionMismatch);
            }
            match receipt.selection_disposition {
                Some(SelectionDisposition::CandidateSelected)
                    if receipt.to_state == Some(LifecycleState::TestDesignReady) =>
                {
                    if !valid_id(receipt.candidate_id.as_deref().unwrap_or_default())
                        || !is_sha256(receipt.design_sha256.as_deref().unwrap_or_default())
                        || receipt.authorization_required.is_none()
                    {
                        return Err(LifecycleError::CandidateNotSelected);
                    }
                }
                Some(SelectionDisposition::Abstain)
                    if receipt.to_state == Some(LifecycleState::AbstainNoAdmissibleTest) =>
                {
                    if receipt.candidate_id.is_some()
                        || receipt.design_sha256.is_some()
                        || receipt.authorization_required.is_some()
                    {
                        return Err(LifecycleError::InvalidReceipt);
                    }
                }
                _ => return Err(LifecycleError::StageOrTransitionMismatch),
            }
        }
        ReceiptStage::ExternalAuthorization => {
            if receipt.from_state != Some(LifecycleState::TestDesignReady)
                || !matches!(
                    receipt.to_state,
                    Some(
                        LifecycleState::AuthorizedForDeclaredTestOnly
                            | LifecycleState::AuthorizationDenied
                    )
                )
                || receipt.authorization_disposition.is_none()
                || !is_sha256(
                    receipt
                        .authorization_scope_sha256
                        .as_deref()
                        .unwrap_or_default(),
                )
            {
                return Err(LifecycleError::StageOrTransitionMismatch);
            }
            let expected = match receipt.to_state {
                Some(LifecycleState::AuthorizedForDeclaredTestOnly) => {
                    AuthorizationDisposition::Authorized
                }
                Some(LifecycleState::AuthorizationDenied) => AuthorizationDisposition::Denied,
                _ => return Err(LifecycleError::StageOrTransitionMismatch),
            };
            if receipt.authorization_disposition != Some(expected) {
                return Err(LifecycleError::StageOrTransitionMismatch);
            }
        }
        ReceiptStage::TestExecution => {
            if !matches!(
                receipt.from_state,
                Some(
                    LifecycleState::AuthorizedForDeclaredTestOnly
                        | LifecycleState::TestDesignReady
                )
            ) || receipt.to_state != Some(LifecycleState::ExecutedAwaitingAdjudication)
                || !valid_id(receipt.candidate_id.as_deref().unwrap_or_default())
                || !is_sha256(receipt.design_sha256.as_deref().unwrap_or_default())
                || !valid_id(receipt.test_attempt_id.as_deref().unwrap_or_default())
                || !is_sha256(
                    receipt
                        .result_manifest_sha256
                        .as_deref()
                        .unwrap_or_default(),
                )
            {
                return Err(LifecycleError::StageOrTransitionMismatch);
            }
        }
        ReceiptStage::ResultAdjudication => {
            if receipt.from_state != Some(LifecycleState::ExecutedAwaitingAdjudication)
                || !matches!(
                    receipt.to_state,
                    Some(
                        LifecycleState::ResultAdjudicated
                            | LifecycleState::ResultRejected
                            | LifecycleState::ResultIndeterminate
                    )
                )
                || receipt.adjudication_disposition.is_none()
                || !is_sha256(
                    receipt
                        .result_manifest_sha256
                        .as_deref()
                        .unwrap_or_default(),
                )
            {
                return Err(LifecycleError::StageOrTransitionMismatch);
            }
            if receipt.to_state == Some(LifecycleState::ResultAdjudicated)
                && !is_sha256(
                    receipt
                        .independent_reference_sha256
                        .as_deref()
                        .unwrap_or_default(),
                )
            {
                return Err(LifecycleError::ResultNotAdjudicated);
            }
            let expected = match receipt.to_state {
                Some(LifecycleState::ResultAdjudicated) => AdjudicationDisposition::Accepted,
                Some(LifecycleState::ResultRejected) => AdjudicationDisposition::Rejected,
                Some(LifecycleState::ResultIndeterminate) => {
                    AdjudicationDisposition::Indeterminate
                }
                _ => return Err(LifecycleError::StageOrTransitionMismatch),
            };
            if receipt.adjudication_disposition != Some(expected) {
                return Err(LifecycleError::StageOrTransitionMismatch);
            }
        }
        ReceiptStage::SubjectBinding | ReceiptStage::TrustAssessment => {}
    }
    Ok(())
}

/// Evaluate one receipt DAG against a frozen aviation lifecycle profile.
///
/// The root is normally a Track T requalification receipt. A shorter root may
/// also represent a historical, indeterminate, denied or abstaining terminal state.
pub fn evaluate_lifecycle(
    receipts: &[LifecycleReceipt],
    root_receipt_id: &str,
    policy: &LifecyclePolicy,
) -> Result<LifecycleResult, LifecycleError> {
    if !valid_id(&policy.claim_id)
        || !is_sha256(&policy.subject_scope_sha256)
        || !is_sha256(&policy.initial_model_scope_sha256)
        || !is_sha256(&policy.target_model_scope_sha256)
        || !is_sha256(&policy.prior_confirmatory_manifest_sha256)
    {
        return Err(LifecycleError::InvalidPolicy);
    }
    if receipts.is_empty() {
        return Err(LifecycleError::ParentMissing);
    }

    let mut by_id: BTreeMap<&str, &LifecycleReceipt> = BTreeMap::new();
    for receipt in receipts {
        if !valid_id(&receipt.receipt_id)
            || !valid_id(&receipt.claim_id)
            || !is_sha256(&receipt.subject_scope_sha256)
            || !is_sha256(&receipt.model_scope_sha256)
            || !is_sha256(&receipt.payload_sha256)
        {
            return Err(LifecycleError::InvalidReceipt);
        }
        if receipt.claim_id != policy.claim_id
            || receipt.subject_scope_sha256 != policy.subject_scope_sha256
        {
            return Err(LifecycleError::SubjectOrClaimMismatch);
        }
        validate_receipt_shape(receipt)?;
        if by_id.insert(receipt.receipt_id.as_str(), receipt).is_some() {
            return Err(LifecycleError::DuplicateReceiptId);
        }
    }

    for receipt in receipts {
        let mut seen = BTreeSet::new();
        for parent in &receipt.parents {
            if !seen.insert(parent.receipt_id.as_str()) {
                return Err(LifecycleError::DuplicateParentEdge);
            }
            let Some(parent_receipt) = by_id.get(parent.receipt_id.as_str()).copied() else {
                return Err(LifecycleError::ParentMissing);
            };
            if !is_sha256(&parent.payload_sha256)
                || parent.payload_sha256 != parent_receipt.payload_sha256
            {
                return Err(LifecycleError::ParentDigestMismatch);
            }
        }
    }

    let Some(root) = by_id.get(root_receipt_id).copied() else {
        return Err(LifecycleError::ParentMissing);
    };
    if root.from_state.is_none() || root.to_state.is_none() {
        return Err(LifecycleError::StageOrTransitionMismatch);
    }

    // Every transition must have exact Track P subject-binding and Track Q trust anchors
    // in its transitive parent graph. Their dispositions are owner-supplied inputs.
    for receipt in receipts.iter().filter(|item| item.from_state.is_some()) {
        let mut ancestors = BTreeSet::new();
        let mut stack: Vec<&str> = receipt
            .parents
            .iter()
            .map(|parent| parent.receipt_id.as_str())
            .collect();
        while let Some(id) = stack.pop() {
            if !ancestors.insert(id) {
                continue;
            }
            let parent = by_id.get(id).copied().ok_or(LifecycleError::ParentMissing)?;
            stack.extend(
                parent
                    .parents
                    .iter()
                    .map(|edge| edge.receipt_id.as_str()),
            );
        }
        let has_subject_anchor = ancestors.iter().any(|id| {
            by_id
                .get(id)
                .is_some_and(|item| item.stage == ReceiptStage::SubjectBinding)
        });
        let has_trust_anchor = ancestors.iter().any(|id| {
            by_id.get(id).is_some_and(|item| {
                item.stage == ReceiptStage::TrustAssessment
                    && item.trust_disposition
                        == Some(TrustDisposition::QualifiedForDeclaredPolicy)
            })
        });
        if !has_subject_anchor || !has_trust_anchor {
            return Err(LifecycleError::RequiredParentMissing);
        }
    }

    for receipt in receipts {
        match receipt.stage {
            ReceiptStage::SubjectBinding => {
                if receipt.model_scope_sha256 != policy.initial_model_scope_sha256 {
                    return Err(LifecycleError::TargetModelScopeMismatch);
                }
            }
            ReceiptStage::TrustAssessment => {
                require_parent_stage(receipt, ReceiptStage::SubjectBinding, &by_id)?;
                if receipt.trust_disposition != Some(TrustDisposition::QualifiedForDeclaredPolicy) {
                    return Err(LifecycleError::RequiredParentMissing);
                }
            }
            ReceiptStage::ModelQualification => {
                if receipt.from_state == Some(LifecycleState::Uninitialized) {
                    if receipt.model_scope_sha256 != policy.initial_model_scope_sha256 {
                        return Err(LifecycleError::TargetModelScopeMismatch);
                    }
                    require_parent_stage(receipt, ReceiptStage::SubjectBinding, &by_id)?;
                    require_parent_stage(receipt, ReceiptStage::TrustAssessment, &by_id)?;
                } else {
                    if receipt.model_scope_sha256 != policy.target_model_scope_sha256 {
                        return Err(LifecycleError::TargetModelScopeMismatch);
                    }
                    let adjudication = receipt
                        .parents
                        .iter()
                        .filter_map(|parent| by_id.get(parent.receipt_id.as_str()).copied())
                        .find(|item| {
                            item.stage == ReceiptStage::ResultAdjudication
                                && item.to_state == Some(LifecycleState::ResultAdjudicated)
                        })
                        .ok_or(LifecycleError::ResultNotAccepted)?;
                    let execution = adjudication
                        .parents
                        .iter()
                        .filter_map(|parent| by_id.get(parent.receipt_id.as_str()).copied())
                        .find(|item| item.stage == ReceiptStage::TestExecution)
                        .ok_or(LifecycleError::RequiredParentMissing)?;
                    if receipt.result_manifest_sha256 != execution.result_manifest_sha256
                        || receipt.prior_confirmatory_manifest_sha256
                            != Some(policy.prior_confirmatory_manifest_sha256.clone())
                    {
                        return Err(LifecycleError::ConfirmatoryManifestNotFresh);
                    }
                    require_parent_stage(receipt, ReceiptStage::ResultAdjudication, &by_id)?;
                }
            }
            ReceiptStage::ChangeAssessment => {
                if receipt.model_scope_sha256 != policy.target_model_scope_sha256 {
                    return Err(LifecycleError::TargetModelScopeMismatch);
                }
                require_parent_stage(receipt, ReceiptStage::ModelQualification, &by_id)?;
                require_parent_stage(receipt, ReceiptStage::TrustAssessment, &by_id)?;
            }
            ReceiptStage::ExperimentSelection => {
                if receipt.model_scope_sha256 != policy.target_model_scope_sha256 {
                    return Err(LifecycleError::TargetModelScopeMismatch);
                }
                let change = receipt
                    .parents
                    .iter()
                    .filter_map(|parent| by_id.get(parent.receipt_id.as_str()).copied())
                    .find(|item| item.stage == ReceiptStage::ChangeAssessment)
                    .ok_or(LifecycleError::RequiredParentMissing)?;
                if change.to_state != Some(LifecycleState::RequalificationRequired) {
                    return Err(LifecycleError::ChangeDispositionNotActionable);
                }
                require_parent_stage(receipt, ReceiptStage::ChangeAssessment, &by_id)?;
            }
            ReceiptStage::ExternalAuthorization => {
                if receipt.model_scope_sha256 != policy.target_model_scope_sha256 {
                    return Err(LifecycleError::TargetModelScopeMismatch);
                }
                let selection = receipt
                    .parents
                    .iter()
                    .filter_map(|parent| by_id.get(parent.receipt_id.as_str()).copied())
                    .find(|item| item.stage == ReceiptStage::ExperimentSelection)
                    .ok_or(LifecycleError::RequiredParentMissing)?;
                if selection.to_state != Some(LifecycleState::TestDesignReady)
                    || receipt.authorization_scope_sha256 != selection.design_sha256
                    || receipt.candidate_id != selection.candidate_id
                {
                    return Err(LifecycleError::AuthorizationNotGrantedForDesign);
                }
            }
            ReceiptStage::TestExecution => {
                if receipt.model_scope_sha256 != policy.target_model_scope_sha256 {
                    return Err(LifecycleError::TargetModelScopeMismatch);
                }
                let selection = receipt
                    .parents
                    .iter()
                    .filter_map(|parent| by_id.get(parent.receipt_id.as_str()).copied())
                    .find(|item| item.stage == ReceiptStage::ExperimentSelection)
                    .ok_or(LifecycleError::RequiredParentMissing)?;
                if selection.to_state != Some(LifecycleState::TestDesignReady)
                    || receipt.candidate_id != selection.candidate_id
                    || receipt.design_sha256 != selection.design_sha256
                {
                    return Err(LifecycleError::ExecutionDoesNotMatchSelection);
                }
                let authorization_required =
                    selection.authorization_required.ok_or(LifecycleError::InvalidReceipt)?;
                let authorization = receipt
                    .parents
                    .iter()
                    .filter_map(|parent| by_id.get(parent.receipt_id.as_str()).copied())
                    .find(|item| item.stage == ReceiptStage::ExternalAuthorization);
                if authorization_required {
                    let auth = authorization.ok_or(LifecycleError::AuthorizationNotGrantedForDesign)?;
                    if auth.authorization_disposition != Some(AuthorizationDisposition::Authorized)
                        || auth.to_state != Some(LifecycleState::AuthorizedForDeclaredTestOnly)
                        || auth.authorization_scope_sha256 != selection.design_sha256
                    {
                        return Err(LifecycleError::AuthorizationNotGrantedForDesign);
                    }
                } else if authorization.is_some_and(|item| {
                    item.authorization_disposition != Some(AuthorizationDisposition::Authorized)
                }) {
                    return Err(LifecycleError::AuthorizationNotGrantedForDesign);
                }
                require_parent_stage(receipt, ReceiptStage::ExperimentSelection, &by_id)?;
            }
            ReceiptStage::ResultAdjudication => {
                if receipt.model_scope_sha256 != policy.target_model_scope_sha256 {
                    return Err(LifecycleError::TargetModelScopeMismatch);
                }
                let execution = receipt
                    .parents
                    .iter()
                    .filter_map(|parent| by_id.get(parent.receipt_id.as_str()).copied())
                    .find(|item| item.stage == ReceiptStage::TestExecution)
                    .ok_or(LifecycleError::RequiredParentMissing)?;
                if receipt.result_manifest_sha256 != execution.result_manifest_sha256 {
                    return Err(LifecycleError::ResultNotAccepted);
                }
                require_parent_stage(receipt, ReceiptStage::TestExecution, &by_id)?;
            }
        }
    }

    // Iterative rooted DFS: detect cycles, canonicalize IDs and reject orphan receipts.
    let mut colors: BTreeMap<&str, u8> = BTreeMap::new();
    let mut reached = BTreeSet::new();
    let mut stack: Vec<(&str, bool)> = vec![(root_receipt_id, false)];
    while let Some((id, exiting)) = stack.pop() {
        if exiting {
            colors.insert(id, 2);
            continue;
        }
        match colors.get(id).copied().unwrap_or(0) {
            1 => return Err(LifecycleError::Cycle),
            2 => continue,
            _ => {}
        }
        colors.insert(id, 1);
        reached.insert(id);
        stack.push((id, true));
        let receipt = by_id.get(id).copied().ok_or(LifecycleError::ParentMissing)?;
        let mut parents: Vec<&str> = receipt
            .parents
            .iter()
            .map(|parent| parent.receipt_id.as_str())
            .collect();
        parents.sort_unstable();
        for parent in parents.into_iter().rev() {
            if colors.get(parent).copied() == Some(1) {
                return Err(LifecycleError::Cycle);
            }
            if colors.get(parent).copied() != Some(2) {
                stack.push((parent, false));
            }
        }
    }
    if reached.len() != receipts.len() {
        return Err(LifecycleError::OrphanReceipt);
    }

    let terminal_state = root.to_state.ok_or(LifecycleError::StageOrTransitionMismatch)?;
    let candidate_id = receipts
        .iter()
        .find(|item| item.stage == ReceiptStage::ExperimentSelection)
        .and_then(|item| item.candidate_id.clone());
    let test_attempt_id = receipts
        .iter()
        .find(|item| item.stage == ReceiptStage::TestExecution)
        .and_then(|item| item.test_attempt_id.clone());
    let accepted_confirmatory_manifest_sha256 = receipts
        .iter()
        .find(|item| {
            item.stage == ReceiptStage::ModelQualification
                && item.to_state == Some(LifecycleState::RequalificationAccepted)
        })
        .and_then(|item| item.confirmatory_manifest_sha256.clone());

    let mut receipt_ids: Vec<String> = reached.into_iter().map(str::to_owned).collect();
    receipt_ids.sort_unstable();
    Ok(LifecycleResult {
        terminal_state,
        receipt_ids,
        candidate_id,
        test_attempt_id,
        accepted_confirmatory_manifest_sha256,
    })
}
