use aeroweift_evidence_lifecycle::{
    evaluate_lifecycle, AdjudicationDisposition, AuthorizationDisposition, ChangeDisposition,
    LifecycleError, LifecyclePolicy, LifecycleReceipt, LifecycleState, ParentRef, ReceiptStage,
    SelectionDisposition, TrustDisposition,
};

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn policy() -> LifecyclePolicy {
    LifecyclePolicy {
        claim_id: "claim-aero-001".into(),
        subject_scope_sha256: digest('c'),
        initial_model_scope_sha256: digest('a'),
        target_model_scope_sha256: digest('b'),
        prior_confirmatory_manifest_sha256: digest('d'),
    }
}

fn receipt(id: &str, stage: ReceiptStage, model_digest: char, parents: Vec<LifecycleReceipt>) -> LifecycleReceipt {
    LifecycleReceipt {
        receipt_id: id.into(),
        claim_id: policy().claim_id,
        stage,
        subject_scope_sha256: digest('c'),
        model_scope_sha256: digest(model_digest),
        payload_sha256: digest(match id {
            "subject" => '1',
            "trust" => '2',
            "initial-t" => '3',
            "change-u" => '4',
            "selection-v" => '5',
            "authorization" => '6',
            "execution" => '7',
            "adjudication" => '8',
            "requalification-t" => '9',
            _ => '0',
        }),
        parents: parents
            .iter()
            .map(|parent| ParentRef {
                receipt_id: parent.receipt_id.clone(),
                payload_sha256: parent.payload_sha256.clone(),
            })
            .collect(),
        from_state: None,
        to_state: None,
        trust_disposition: None,
        change_disposition: None,
        selection_disposition: None,
        authorization_disposition: None,
        authorization_required: None,
        candidate_id: None,
        design_sha256: None,
        authorization_scope_sha256: None,
        test_attempt_id: None,
        result_manifest_sha256: None,
        independent_reference_sha256: None,
        confirmatory_manifest_sha256: None,
        prior_confirmatory_manifest_sha256: None,
        adjudication_disposition: None,
    }
}

fn graph() -> Vec<LifecycleReceipt> {
    let subject = receipt("subject", ReceiptStage::SubjectBinding, 'a', vec![]);

    let mut trust = receipt(
        "trust",
        ReceiptStage::TrustAssessment,
        'a',
        vec![subject.clone()],
    );
    trust.trust_disposition = Some(TrustDisposition::QualifiedForDeclaredPolicy);

    let mut initial = receipt(
        "initial-t",
        ReceiptStage::ModelQualification,
        'a',
        vec![subject.clone(), trust.clone()],
    );
    initial.from_state = Some(LifecycleState::Uninitialized);
    initial.to_state = Some(LifecycleState::QualifiedCurrent);
    initial.confirmatory_manifest_sha256 = Some(digest('d'));

    let mut change = receipt(
        "change-u",
        ReceiptStage::ChangeAssessment,
        'b',
        vec![initial.clone(), trust.clone()],
    );
    change.from_state = Some(LifecycleState::QualifiedCurrent);
    change.to_state = Some(LifecycleState::RequalificationRequired);
    change.change_disposition = Some(ChangeDisposition::RequalificationRequired);

    let mut selection = receipt(
        "selection-v",
        ReceiptStage::ExperimentSelection,
        'b',
        vec![change.clone()],
    );
    selection.from_state = Some(LifecycleState::RequalificationRequired);
    selection.to_state = Some(LifecycleState::TestDesignReady);
    selection.selection_disposition = Some(SelectionDisposition::CandidateSelected);
    selection.authorization_required = Some(true);
    selection.candidate_id = Some("candidate-crosswind-07".into());
    selection.design_sha256 = Some(digest('e'));

    let mut authorization = receipt(
        "authorization",
        ReceiptStage::ExternalAuthorization,
        'b',
        vec![selection.clone()],
    );
    authorization.from_state = Some(LifecycleState::TestDesignReady);
    authorization.to_state = Some(LifecycleState::AuthorizedForDeclaredTestOnly);
    authorization.authorization_disposition = Some(AuthorizationDisposition::Authorized);
    authorization.authorization_scope_sha256 = Some(digest('e'));
    authorization.candidate_id = Some("candidate-crosswind-07".into());

    let mut execution = receipt(
        "execution",
        ReceiptStage::TestExecution,
        'b',
        vec![selection.clone(), authorization.clone()],
    );
    execution.from_state = Some(LifecycleState::AuthorizedForDeclaredTestOnly);
    execution.to_state = Some(LifecycleState::ExecutedAwaitingAdjudication);
    execution.candidate_id = Some("candidate-crosswind-07".into());
    execution.design_sha256 = Some(digest('e'));
    execution.test_attempt_id = Some("attempt-001".into());
    execution.result_manifest_sha256 = Some(digest('f'));

    let mut adjudication = receipt(
        "adjudication",
        ReceiptStage::ResultAdjudication,
        'b',
        vec![execution.clone()],
    );
    adjudication.from_state = Some(LifecycleState::ExecutedAwaitingAdjudication);
    adjudication.to_state = Some(LifecycleState::ResultAdjudicated);
    adjudication.adjudication_disposition = Some(AdjudicationDisposition::Accepted);
    adjudication.result_manifest_sha256 = Some(digest('f'));
    adjudication.independent_reference_sha256 = Some(digest('0'));

    let mut requalification = receipt(
        "requalification-t",
        ReceiptStage::ModelQualification,
        'b',
        vec![adjudication.clone()],
    );
    requalification.from_state = Some(LifecycleState::ResultAdjudicated);
    requalification.to_state = Some(LifecycleState::RequalificationAccepted);
    requalification.result_manifest_sha256 = Some(digest('f'));
    requalification.confirmatory_manifest_sha256 = Some(digest('9'));
    requalification.prior_confirmatory_manifest_sha256 = Some(digest('d'));

    vec![
        subject,
        trust,
        initial,
        change,
        selection,
        authorization,
        execution,
        adjudication,
        requalification,
    ]
}

fn assess(receipts: &[LifecycleReceipt]) -> Result<aeroweift_evidence_lifecycle::LifecycleResult, LifecycleError> {
    evaluate_lifecycle(receipts, "requalification-t", &policy())
}

fn receipt_mut<'a>(receipts: &'a mut [LifecycleReceipt], id: &str) -> &'a mut LifecycleReceipt {
    receipts
        .iter_mut()
        .find(|item| item.receipt_id == id)
        .unwrap()
}

#[test]
fn positive_control_closes_the_full_qualification_lifecycle() {
    let result = assess(&graph()).unwrap();
    assert_eq!(result.terminal_state, LifecycleState::RequalificationAccepted);
    assert_eq!(result.receipt_ids.len(), 9);
    assert_eq!(
        result.candidate_id,
        Some("candidate-crosswind-07".to_string())
    );
    assert_eq!(result.test_attempt_id, Some("attempt-001".to_string()));
    assert_eq!(
        result.accepted_confirmatory_manifest_sha256,
        Some(digest('9'))
    );
}

#[test]
fn a_track_v_recommendation_cannot_skip_external_authorization() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "execution")
        .parents
        .retain(|parent| parent.receipt_id != "authorization");
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::AuthorizationNotGrantedForDesign)
    );
}

#[test]
fn authorization_for_a_different_design_is_rejected() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "authorization").authorization_scope_sha256 =
        Some(digest('0'));
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::AuthorizationNotGrantedForDesign)
    );
}

#[test]
fn execution_for_a_different_candidate_is_rejected() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "execution").candidate_id =
        Some("candidate-other".into());
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::ExecutionDoesNotMatchSelection)
    );
}

#[test]
fn execution_for_a_modified_design_is_rejected() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "execution").design_sha256 = Some(digest('0'));
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::ExecutionDoesNotMatchSelection)
    );
}

#[test]
fn a_denied_authorization_cannot_be_followed_by_execution() {
    let mut receipts = graph();
    let authorization = receipt_mut(&mut receipts, "authorization");
    authorization.to_state = Some(LifecycleState::AuthorizationDenied);
    authorization.authorization_disposition = Some(AuthorizationDisposition::Denied);
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::AuthorizationNotGrantedForDesign)
    );
}

#[test]
fn an_abstaining_track_v_candidate_cannot_be_executed() {
    let mut receipts = graph();
    let selection = receipt_mut(&mut receipts, "selection");
    selection.to_state = Some(LifecycleState::AbstainNoAdmissibleTest);
    selection.selection_disposition = Some(SelectionDisposition::Abstain);
    selection.authorization_required = None;
    selection.candidate_id = None;
    selection.design_sha256 = None;
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::CandidateNotSelected)
    );
}

#[test]
fn execution_is_not_model_requalification_without_result_adjudication() {
    let mut receipts = graph();
    receipts.retain(|item| item.stage != ReceiptStage::ResultAdjudication);
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::ParentMissing)
    );
}

#[test]
fn accepted_adjudication_requires_independent_reference_evidence() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "adjudication").independent_reference_sha256 = None;
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::ResultNotAdjudicated)
    );
}

#[test]
fn adjudication_must_bind_the_exact_execution_result_manifest() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "adjudication").result_manifest_sha256 =
        Some(digest('0'));
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::ResultNotAccepted)
    );
}

#[test]
fn requalification_must_use_a_fresh_confirmatory_manifest() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "requalification-t").confirmatory_manifest_sha256 =
        Some(digest('d'));
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::ConfirmatoryManifestNotFresh)
    );
}

#[test]
fn requalification_must_use_the_exact_target_model_scope() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "requalification-t").model_scope_sha256 =
        digest('a');
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::TargetModelScopeMismatch)
    );
}

#[test]
fn an_initial_qualification_cannot_be_minted_by_a_test_execution_receipt() {
    let mut receipts = graph();
    let requalification = receipt_mut(&mut receipts, "requalification-t");
    requalification.from_state = Some(LifecycleState::ExecutedAwaitingAdjudication);
    requalification.to_state = Some(LifecycleState::RequalificationAccepted);
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::EvidenceStageCannotQualifyModel)
    );
}

#[test]
fn track_u_indeterminate_status_cannot_be_promoted_to_test_design_ready() {
    let mut receipts = graph();
    let change = receipt_mut(&mut receipts, "change-u");
    change.to_state = Some(LifecycleState::Indeterminate);
    change.change_disposition = Some(ChangeDisposition::Indeterminate);
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::ChangeDispositionNotActionable)
    );
}

#[test]
fn missing_track_q_trust_anchor_blocks_initial_qualification() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "initial-t")
        .parents
        .retain(|parent| parent.receipt_id != "trust");
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::RequiredParentMissing)
    );
}

#[test]
fn unqualified_track_q_anchor_cannot_support_a_positive_lifecycle() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "trust").trust_disposition =
        Some(TrustDisposition::Indeterminate);
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::RequiredParentMissing)
    );
}

#[test]
fn mismatched_subject_scope_is_rejected_across_the_chain() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "execution").subject_scope_sha256 = digest('0');
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::SubjectOrClaimMismatch)
    );
}

#[test]
fn mismatched_claim_identity_is_rejected_across_the_chain() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "adjudication").claim_id = "other-claim".into();
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::SubjectOrClaimMismatch)
    );
}

#[test]
fn parent_payload_digest_substitution_is_rejected() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "requalification-t").parents[0].payload_sha256 =
        digest('0');
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::ParentDigestMismatch)
    );
}

#[test]
fn orphan_receipts_cannot_be_silently_included() {
    let mut receipts = graph();
    let mut orphan = receipt(
        "orphan-subject",
        ReceiptStage::SubjectBinding,
        'a',
        vec![],
    );
    orphan.payload_sha256 = digest('0');
    receipts.push(orphan);
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::OrphanReceipt)
    );
}

#[test]
fn duplicate_receipt_ids_are_rejected() {
    let mut receipts = graph();
    receipts.push(receipts[0].clone());
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::DuplicateReceiptId)
    );
}

#[test]
fn duplicate_parent_edges_are_rejected() {
    let mut receipts = graph();
    let edge = receipt_mut(&mut receipts, "requalification-t").parents[0].clone();
    receipt_mut(&mut receipts, "requalification-t")
        .parents
        .push(edge);
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::DuplicateParentEdge)
    );
}

#[test]
fn receipt_graph_cycles_are_rejected() {
    let mut receipts = graph();
    let initial = receipt_mut(&mut receipts, "initial-t");
    let edge = ParentRef {
        receipt_id: "change-u".into(),
        payload_sha256: digest('4'),
    };
    initial.parents.push(edge);
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::Cycle)
    );
}

#[test]
fn receipt_input_permutation_preserves_exact_replay_result() {
    let original = graph();
    let expected = assess(&original).unwrap();
    let mut permuted = original;
    permuted.reverse();
    for receipt in &mut permuted {
        receipt.parents.reverse();
    }
    assert_eq!(assess(&permuted).unwrap(), expected);
}

#[test]
fn external_authorization_scope_is_the_selected_design_not_the_model_scope() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "authorization").authorization_scope_sha256 =
        Some(digest('b'));
    assert_eq!(
        assess(&receipts),
        Err(LifecycleError::AuthorizationNotGrantedForDesign)
    );
}

#[test]
fn a_short_lifecycle_can_stop_at_u_indeterminate_without_promoting_a_model() {
    let mut receipts = graph();
    receipts.retain(|item| {
        matches!(
            item.stage,
            ReceiptStage::SubjectBinding
                | ReceiptStage::TrustAssessment
                | ReceiptStage::ModelQualification
                | ReceiptStage::ChangeAssessment
        )
    });
    let change = receipt_mut(&mut receipts, "change-u");
    change.to_state = Some(LifecycleState::Indeterminate);
    change.change_disposition = Some(ChangeDisposition::Indeterminate);
    let result = evaluate_lifecycle(&receipts, "change-u", &policy()).unwrap();
    assert_eq!(result.terminal_state, LifecycleState::Indeterminate);
}

#[test]
fn a_short_lifecycle_can_abstain_without_creating_authority_or_execution() {
    let mut receipts = graph();
    receipts.retain(|item| {
        matches!(
            item.stage,
            ReceiptStage::SubjectBinding
                | ReceiptStage::TrustAssessment
                | ReceiptStage::ModelQualification
                | ReceiptStage::ChangeAssessment
                | ReceiptStage::ExperimentSelection
        )
    });
    let selection = receipt_mut(&mut receipts, "selection-v");
    selection.to_state = Some(LifecycleState::AbstainNoAdmissibleTest);
    selection.selection_disposition = Some(SelectionDisposition::Abstain);
    selection.authorization_required = None;
    selection.candidate_id = None;
    selection.design_sha256 = None;
    let result = evaluate_lifecycle(&receipts, "selection-v", &policy()).unwrap();
    assert_eq!(result.terminal_state, LifecycleState::AbstainNoAdmissibleTest);
    assert_eq!(result.candidate_id, None);
    assert_eq!(result.test_attempt_id, None);
}
