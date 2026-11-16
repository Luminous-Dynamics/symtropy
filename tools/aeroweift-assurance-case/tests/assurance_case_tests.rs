use aeroweift_assurance_case::{
    AssumptionData, AssumptionState, AssuranceCasePolicy, AssuranceCaseScope, AssuranceEdge,
    AssuranceNode, AssuranceStatus, AuthorityDecisionData, AuthorityDecisionStatus,
    AuthorityRequirement, BlockerReason, ClaimData, ClaimEvidenceRequirement, ClaimState,
    DefeaterData, DefeaterState, EdgeKind, EvaluationError, EvidenceDisposition, EvidencePurpose,
    EvidenceReference, EvidenceSourceKind, MitigationEvidenceRequirement, NodeKind,
    evaluate_assurance_case,
};

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn scope() -> AssuranceCaseScope {
    AssuranceCaseScope {
        case_id: "case-aero-001".into(),
        campaign_sha256: digest('a'),
        operation_id: "operation-001".into(),
        aircraft_id: "aircraft-pseudo-001".into(),
        flight_id: "flight-001".into(),
        authorization_id: "authorization-001".into(),
        authorization_revision: 3,
        intent_sha256: digest('b'),
        profile_sha256: digest('c'),
        operating_domain_sha256: digest('d'),
        time_scale_id: "UTC-SYNTHETIC".into(),
        horizontal_frame_id: "LOCAL-ENU-01".into(),
        vertical_datum_id: "SYNTHETIC-VD-01".into(),
        operation_start: 20,
        operation_end: 100,
        effective_cutoff: 50,
        knowledge_cutoff: 60,
    }
}

fn policy() -> AssuranceCasePolicy {
    AssuranceCasePolicy {
        expected_scope: scope(),
        required_claim_evidence: vec![
            ClaimEvidenceRequirement {
                claim_id: "root".into(),
                required_purposes: vec![],
                minimum_independent_evidence_groups: 0,
            },
            ClaimEvidenceRequirement {
                claim_id: "claim-model-envelope".into(),
                required_purposes: vec![EvidencePurpose::ModelCredibility],
                minimum_independent_evidence_groups: 1,
            },
        ],
        required_mitigation_evidence: vec![MitigationEvidenceRequirement {
            mitigation_id: "mitigation-conflict-screen".into(),
            required_purposes: vec![EvidencePurpose::PhysicalMitigationEffectiveness],
            minimum_independent_evidence_groups: 1,
        }],
        policy_sha256: digest('e'),
        verifier_build_sha256: digest('f'),
        required_hazards: vec!["collision-avoidance".into()],
        required_authorities: vec![AuthorityRequirement {
            role: "risk-acceptance".into(),
            authority_id: "competent-authority".into(),
        }],
    }
}

fn node(id: &str, kind: NodeKind) -> AssuranceNode {
    AssuranceNode {
        node_id: id.into(),
        kind,
        payload_sha256: digest('1'),
        scope: scope(),
        owner_role: format!("owner-{id}"),
        revision: "rev-1".into(),
        valid_from: 0,
        valid_until: 110,
        claim: None,
        assumption: None,
        hazard_id: None,
        mitigation_state: None,
        evidence: None,
        defeater: None,
        authority_decision: None,
    }
}

fn claim_node(id: &str, kind: NodeKind, state: ClaimState, minimum_groups: usize) -> AssuranceNode {
    let mut result = node(id, kind);
    result.claim = Some(ClaimData {
        state,
        minimum_independent_evidence_groups: minimum_groups,
    });
    result
}

fn evidence_node(
    id: &str,
    source_kind: EvidenceSourceKind,
    disposition: EvidenceDisposition,
    correlation_group: Option<&str>,
) -> AssuranceNode {
    let mut result = node(id, NodeKind::Evidence);
    result.evidence = Some(EvidenceReference {
        source_receipt_id: format!("receipt-{id}"),
        source_payload_sha256: digest('2'),
        source_assessment_sha256: digest('3'),
        source_kind,
        purpose: match source_kind {
            EvidenceSourceKind::TrackL => EvidencePurpose::PntReachability,
            EvidenceSourceKind::TrackM => EvidencePurpose::SurveillanceObservability,
            EvidenceSourceKind::TrackN => EvidencePurpose::IntentConformance,
            EvidenceSourceKind::TrackP => EvidencePurpose::CausalReceiptBinding,
            EvidenceSourceKind::TrackQ => EvidencePurpose::AuthenticatedCustody,
            EvidenceSourceKind::TrackR => EvidencePurpose::ConflictPerformance,
            EvidenceSourceKind::TrackS => EvidencePurpose::MultiSubjectScope,
            EvidenceSourceKind::TrackT => EvidencePurpose::ModelCredibility,
            EvidenceSourceKind::TrackU => EvidencePurpose::ModelRequalification,
            EvidenceSourceKind::TrackVPlanning => EvidencePurpose::ExperimentPlan,
            EvidenceSourceKind::ExternalFlightTest => {
                EvidencePurpose::PhysicalMitigationEffectiveness
            }
            EvidenceSourceKind::ExternalMeasurement => EvidencePurpose::ReferenceMeasurement,
            EvidenceSourceKind::ExternalAuthority => EvidencePurpose::ExternalAuthorityRecord,
            EvidenceSourceKind::GenericSoftwareCi => EvidencePurpose::GenericSoftwareVerification,
            EvidenceSourceKind::Other => EvidencePurpose::Unclassified,
        },
        disposition,
        correlation_group_id: correlation_group.map(str::to_owned),
        effective_cutoff: 50,
        knowledge_cutoff: 60,
    });
    result
}

fn decision_node(status: AuthorityDecisionStatus) -> AssuranceNode {
    let mut result = node("authority-decision", NodeKind::AuthorityDecision);
    result.authority_decision = Some(AuthorityDecisionData {
        role: "risk-acceptance".into(),
        authority_id: "competent-authority".into(),
        status,
        effective_at: 10,
        recorded_at: 12,
        valid_until: Some(110),
        conditions_sha256: digest('4'),
    });
    result
}

fn edge(from: &str, to: &str, kind: EdgeKind) -> AssuranceEdge {
    AssuranceEdge {
        from_node_id: from.into(),
        to_node_id: to.into(),
        kind,
    }
}

fn positive_case() -> (Vec<AssuranceNode>, Vec<AssuranceEdge>) {
    let mut root = claim_node("root", NodeKind::TopClaim, ClaimState::Supported, 0);
    root.payload_sha256 = digest('5');

    let strategy = node("strategy", NodeKind::Strategy);
    let claim = claim_node(
        "claim-model-envelope",
        NodeKind::Claim,
        ClaimState::Supported,
        1,
    );
    let context = node("context-operation", NodeKind::Context);

    let mut assumption = node("assumption-weather-profile", NodeKind::Assumption);
    assumption.assumption = Some(AssumptionData {
        state: AssumptionState::Supported,
        assumption_id: "assumption-weather-001".into(),
    });

    let mut hazard = node("hazard-collision", NodeKind::Hazard);
    hazard.hazard_id = Some("collision-avoidance".into());

    let mut mitigation = node("mitigation-conflict-screen", NodeKind::Mitigation);
    mitigation.mitigation_state = Some(ClaimState::Supported);

    let evidence_model = evidence_node(
        "evidence-model-validation",
        EvidenceSourceKind::TrackT,
        EvidenceDisposition::Qualified,
        Some("controlled-flight-campaign-01"),
    );
    let evidence_mitigation = evidence_node(
        "evidence-mitigation-test",
        EvidenceSourceKind::ExternalFlightTest,
        EvidenceDisposition::Qualified,
        Some("independent-test-lab-01"),
    );
    let evidence_defeater = evidence_node(
        "evidence-defeater-resolution",
        EvidenceSourceKind::ExternalMeasurement,
        EvidenceDisposition::Qualified,
        Some("instrument-calibration-01"),
    );

    let mut defeater = node("defeater-sensor-bias", NodeKind::Defeater);
    defeater.defeater = Some(DefeaterData {
        state: DefeaterState::ResolvedWithEvidence,
        defeater_id: "defeater-sensor-bias-001".into(),
    });

    let authority = decision_node(AuthorityDecisionStatus::Accepted);

    let nodes = vec![
        root,
        strategy,
        claim,
        context,
        assumption,
        hazard,
        mitigation,
        evidence_model,
        evidence_mitigation,
        evidence_defeater,
        defeater,
        authority,
    ];
    let edges = vec![
        edge("root", "strategy", EdgeKind::Decomposes),
        edge("strategy", "claim-model-envelope", EdgeKind::Decomposes),
        edge(
            "evidence-model-validation",
            "claim-model-envelope",
            EdgeKind::Supports,
        ),
        edge("context-operation", "root", EdgeKind::Contextualizes),
        edge(
            "evidence-model-validation",
            "assumption-weather-profile",
            EdgeKind::Supports,
        ),
        edge("assumption-weather-profile", "root", EdgeKind::Assumes),
        edge(
            "evidence-mitigation-test",
            "mitigation-conflict-screen",
            EdgeKind::Supports,
        ),
        edge(
            "mitigation-conflict-screen",
            "hazard-collision",
            EdgeKind::Mitigates,
        ),
        edge("strategy", "hazard-collision", EdgeKind::AddressesHazard),
        edge(
            "context-operation",
            "hazard-collision",
            EdgeKind::Contextualizes,
        ),
        edge(
            "hazard-collision",
            "authority-decision",
            EdgeKind::RequiresDecision,
        ),
        edge(
            "evidence-defeater-resolution",
            "defeater-sensor-bias",
            EdgeKind::Discharges,
        ),
        edge("defeater-sensor-bias", "root", EdgeKind::Challenges),
    ];
    (nodes, edges)
}

fn evaluate(
    nodes: &[AssuranceNode],
    edges: &[AssuranceEdge],
) -> Result<aeroweift_assurance_case::AssuranceResult, EvaluationError> {
    evaluate_assurance_case(nodes, edges, "root", &policy())
}

fn blocker(result: &aeroweift_assurance_case::AssuranceResult, reason: BlockerReason) -> bool {
    result.blockers.iter().any(|item| item.reason == reason)
}

#[test]
fn positive_control_is_review_ready_only_and_never_permission() {
    let (nodes, edges) = positive_case();
    let result = evaluate(&nodes, &edges).unwrap();
    assert_eq!(result.status, AssuranceStatus::ReviewReadyOnly);
    assert!(result.blockers.is_empty());
    assert!(!result.permission_granted);
    assert_eq!(result.case_id, "case-aero-001");
    assert_eq!(result.root_claim_id, "root");
    assert_eq!(
        result.independent_evidence_groups_by_claim,
        vec![
            ("claim-model-envelope".to_string(), 1),
            ("root".to_string(), 0)
        ]
    );
    assert_eq!(
        result.independent_evidence_groups_by_mitigation,
        vec![("mitigation-conflict-screen".to_string(), 1)]
    );
}

#[test]
fn missing_root_strategy_blocks_review_readiness() {
    let (nodes, mut edges) = positive_case();
    edges.retain(|item| !(item.from_node_id == "root" && item.kind == EdgeKind::Decomposes));
    let result = evaluate(&nodes, &edges).unwrap();
    assert_eq!(result.status, AssuranceStatus::IncompleteArgument);
    assert!(blocker(
        &result,
        BlockerReason::MissingDecompositionStrategy
    ));
}

#[test]
fn strategy_without_subclaim_blocks_review_readiness() {
    let (nodes, mut edges) = positive_case();
    edges.retain(|item| item.from_node_id != "strategy");
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(&result, BlockerReason::StrategyWithoutSubclaim));
}

#[test]
fn claim_without_support_or_decomposition_is_explicitly_blocked() {
    let (nodes, mut edges) = positive_case();
    edges.retain(|item| {
        !(item.to_node_id == "claim-model-envelope" && item.kind == EdgeKind::Supports)
    });
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(
        &result,
        BlockerReason::ClaimWithoutSupportOrDecomposition
    ));
}

#[test]
fn required_hazard_without_mitigation_blocks_review() {
    let (mut nodes, mut edges) = positive_case();
    nodes.retain(|node| {
        node.node_id != "mitigation-conflict-screen"
            && node.node_id != "evidence-mitigation-test"
    });
    edges.retain(|edge| {
        edge.from_node_id != "mitigation-conflict-screen"
            && edge.to_node_id != "mitigation-conflict-screen"
            && edge.from_node_id != "evidence-mitigation-test"
            && edge.to_node_id != "evidence-mitigation-test"
    });
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(&result, BlockerReason::HazardWithoutMitigation));
}

#[test]
fn missing_required_hazard_node_is_not_silently_omitted() {
    let (nodes, edges) = positive_case();
    let mut frozen = policy();
    frozen.required_hazards.push("loss-of-control".into());
    let result = evaluate_assurance_case(&nodes, &edges, "root", &frozen).unwrap();
    assert!(blocker(&result, BlockerReason::MissingRequiredHazard));
}

#[test]
fn mitigation_without_effectiveness_evidence_is_blocked() {
    let (mut nodes, mut edges) = positive_case();
    nodes.retain(|node| node.node_id != "evidence-mitigation-test");
    edges.retain(|edge| {
        edge.from_node_id != "evidence-mitigation-test"
            && edge.to_node_id != "evidence-mitigation-test"
    });
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(&result, BlockerReason::MitigationWithoutEvidence));
}

#[test]
fn unsupported_mitigation_claim_is_blocked() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "mitigation-conflict-screen")
        .unwrap()
        .mitigation_state = Some(ClaimState::Unsupported);
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(&result, BlockerReason::MitigationNotSupported));
}

#[test]
fn open_defeater_blocks_even_when_other_claims_are_positive() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "defeater-sensor-bias")
        .unwrap()
        .defeater
        .as_mut()
        .unwrap()
        .state = DefeaterState::Open;
    let result = evaluate(&nodes, &edges).unwrap();
    assert_eq!(result.status, AssuranceStatus::BlockedByOpenDefeater);
    assert!(blocker(&result, BlockerReason::OpenDefeater));
}

#[test]
fn resolved_defeater_without_qualified_discharge_is_blocked() {
    let (mut nodes, mut edges) = positive_case();
    nodes.retain(|node| node.node_id != "evidence-defeater-resolution");
    edges.retain(|edge| {
        edge.kind != EdgeKind::Discharges
            && edge.from_node_id != "evidence-defeater-resolution"
            && edge.to_node_id != "evidence-defeater-resolution"
    });
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(&result, BlockerReason::DefeaterWithoutDischarge));
}

#[test]
fn unknown_assumption_blocks_review_readiness() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "assumption-weather-profile")
        .unwrap()
        .assumption
        .as_mut()
        .unwrap()
        .state = AssumptionState::Unknown;
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(&result, BlockerReason::AssumptionUnknown));
    assert_eq!(result.status, AssuranceStatus::Indeterminate);
}

#[test]
fn historical_evidence_cannot_be_used_as_current_support() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "evidence-model-validation")
        .unwrap()
        .evidence
        .as_mut()
        .unwrap()
        .disposition = EvidenceDisposition::HistoricalOnly;
    let result = evaluate(&nodes, &edges).unwrap();
    assert_eq!(
        result.status,
        AssuranceStatus::BlockedByUnsupportedOrStaleEvidence
    );
    assert!(blocker(&result, BlockerReason::EvidenceNotCurrent));
}

#[test]
fn requalification_required_evidence_cannot_remain_positive() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "evidence-model-validation")
        .unwrap()
        .evidence
        .as_mut()
        .unwrap()
        .disposition = EvidenceDisposition::RequalificationRequired;
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(&result, BlockerReason::EvidenceNotCurrent));
}

#[test]
fn generic_ci_receipt_is_not_operational_evidence() {
    let (mut nodes, mut edges) = positive_case();
    let ci = evidence_node(
        "evidence-generic-ci",
        EvidenceSourceKind::GenericSoftwareCi,
        EvidenceDisposition::Qualified,
        Some("ci-run-001"),
    );
    nodes.push(ci);
    edges.push(edge(
        "evidence-generic-ci",
        "claim-model-envelope",
        EdgeKind::Supports,
    ));
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(&result, BlockerReason::NonOperationalEvidence));
}

#[test]
fn track_v_planning_receipt_is_not_promoted_to_realized_validation_evidence() {
    let (mut nodes, mut edges) = positive_case();
    nodes.push(evidence_node(
        "evidence-experiment-plan",
        EvidenceSourceKind::TrackVPlanning,
        EvidenceDisposition::Qualified,
        Some("planning-output-01"),
    ));
    edges.push(edge(
        "evidence-experiment-plan",
        "claim-model-envelope",
        EdgeKind::Supports,
    ));
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(&result, BlockerReason::NonOperationalEvidence));
}

#[test]
fn unknown_evidence_source_is_fail_closed() {
    let (mut nodes, mut edges) = positive_case();
    nodes.push(evidence_node(
        "evidence-other-unknown",
        EvidenceSourceKind::Other,
        EvidenceDisposition::Qualified,
        Some("unknown-owner-01"),
    ));
    edges.push(edge(
        "evidence-other-unknown",
        "claim-model-envelope",
        EdgeKind::Supports,
    ));
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(&result, BlockerReason::NonOperationalEvidence));
}

#[test]
fn evidence_from_different_aircraft_scope_is_rejected() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "evidence-model-validation")
        .unwrap()
        .scope
        .aircraft_id = "other-aircraft".into();
    assert_eq!(
        evaluate(&nodes, &edges),
        Err(EvaluationError::ScopeMismatch)
    );
}

#[test]
fn evidence_cutoff_must_match_the_exact_case_cutoffs() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "evidence-model-validation")
        .unwrap()
        .evidence
        .as_mut()
        .unwrap()
        .knowledge_cutoff = 59;
    assert_eq!(
        evaluate(&nodes, &edges),
        Err(EvaluationError::EvidenceCutoffMismatch)
    );
}

#[test]
fn duplicate_correlation_group_does_not_count_as_independent_evidence() {
    let (mut nodes, mut edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "claim-model-envelope")
        .unwrap()
        .claim
        .as_mut()
        .unwrap()
        .minimum_independent_evidence_groups = 2;
    nodes.push(evidence_node(
        "evidence-model-correlated-copy",
        EvidenceSourceKind::ExternalMeasurement,
        EvidenceDisposition::Qualified,
        Some("controlled-flight-campaign-01"),
    ));
    edges.push(edge(
        "evidence-model-correlated-copy",
        "claim-model-envelope",
        EdgeKind::Supports,
    ));
    let result = evaluate(&nodes, &edges).unwrap();
    assert_eq!(result.status, AssuranceStatus::Indeterminate);
    assert!(blocker(
        &result,
        BlockerReason::InsufficientIndependentEvidenceGroups
    ));
    assert_eq!(
        result
            .independent_evidence_groups_by_claim
            .iter()
            .find(|(id, _)| id == "claim-model-envelope")
            .unwrap()
            .1,
        1
    );
}

#[test]
fn evidence_with_unknown_correlation_does_not_add_independence_credit() {
    let (mut nodes, mut edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "claim-model-envelope")
        .unwrap()
        .claim
        .as_mut()
        .unwrap()
        .minimum_independent_evidence_groups = 2;
    nodes.push(evidence_node(
        "evidence-correlation-unknown",
        EvidenceSourceKind::ExternalMeasurement,
        EvidenceDisposition::Qualified,
        None,
    ));
    edges.push(edge(
        "evidence-correlation-unknown",
        "claim-model-envelope",
        EdgeKind::Supports,
    ));
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(
        &result,
        BlockerReason::InsufficientIndependentEvidenceGroups
    ));
}

#[test]
fn missing_external_authority_decision_is_not_inferred() {
    let (mut nodes, mut edges) = positive_case();
    nodes.retain(|item| item.kind != NodeKind::AuthorityDecision);
    edges.retain(|edge| edge.to_node_id != "authority-decision");
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(&result, BlockerReason::MissingAuthorityDecision));
    assert_eq!(
        result.outstanding_authority_roles,
        vec!["risk-acceptance".to_string()]
    );
}

#[test]
fn pending_authority_decision_blocks_review_ready() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.kind == NodeKind::AuthorityDecision)
        .unwrap()
        .authority_decision
        .as_mut()
        .unwrap()
        .status = AuthorityDecisionStatus::Pending;
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(
        &result,
        BlockerReason::AuthorityDecisionOutstanding
    ));
    assert!(!result.permission_granted);
}

#[test]
fn rejected_authority_decision_is_not_accepted_as_risk_approval() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.kind == NodeKind::AuthorityDecision)
        .unwrap()
        .authority_decision
        .as_mut()
        .unwrap()
        .status = AuthorityDecisionStatus::Rejected;
    let result = evaluate(&nodes, &edges).unwrap();
    assert_eq!(
        result.status,
        AssuranceStatus::BlockedByUnsupportedOrStaleEvidence
    );
    assert!(blocker(
        &result,
        BlockerReason::AuthorityDecisionRejectedOrExpired
    ));
}

#[test]
fn authority_decision_recorded_after_knowledge_cutoff_is_not_used() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.kind == NodeKind::AuthorityDecision)
        .unwrap()
        .authority_decision
        .as_mut()
        .unwrap()
        .recorded_at = 70;
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(
        &result,
        BlockerReason::AuthorityDecisionOutstanding
    ));
    assert_eq!(
        result.outstanding_authority_roles,
        vec!["risk-acceptance".to_string()]
    );
}

#[test]
fn accepted_assumption_must_reference_a_current_policy_pinned_authority_decision() {
    let (mut nodes, mut edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "assumption-weather-profile")
        .unwrap()
        .assumption
        .as_mut()
        .unwrap()
        .state = AssumptionState::AcceptedByAuthority;
    edges.push(edge(
        "assumption-weather-profile",
        "authority-decision",
        EdgeKind::RequiresDecision,
    ));
    let result = evaluate(&nodes, &edges).unwrap();
    assert_eq!(result.status, AssuranceStatus::ReviewReadyOnly);
}

#[test]
fn accepted_assumption_cannot_rely_on_decision_not_yet_known() {
    let (mut nodes, mut edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "assumption-weather-profile")
        .unwrap()
        .assumption
        .as_mut()
        .unwrap()
        .state = AssumptionState::AcceptedByAuthority;
    nodes
        .iter_mut()
        .find(|item| item.kind == NodeKind::AuthorityDecision)
        .unwrap()
        .authority_decision
        .as_mut()
        .unwrap()
        .recorded_at = 70;
    edges.push(edge(
        "assumption-weather-profile",
        "authority-decision",
        EdgeKind::RequiresDecision,
    ));
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(&result, BlockerReason::AssumptionWithoutSupport));
    assert!(blocker(
        &result,
        BlockerReason::AuthorityDecisionOutstanding
    ));
}

#[test]
fn duplicate_node_identity_is_rejected() {
    let (mut nodes, edges) = positive_case();
    nodes.push(nodes[0].clone());
    assert_eq!(
        evaluate(&nodes, &edges),
        Err(EvaluationError::DuplicateNodeId)
    );
}

#[test]
fn missing_edge_endpoint_is_rejected() {
    let (nodes, mut edges) = positive_case();
    edges.push(edge("missing-node", "root", EdgeKind::Contextualizes));
    assert_eq!(evaluate(&nodes, &edges), Err(EvaluationError::MissingNode));
}

#[test]
fn decomposes_cycle_is_rejected() {
    let (nodes, mut edges) = positive_case();
    edges.push(edge(
        "claim-model-envelope",
        "strategy",
        EdgeKind::Decomposes,
    ));
    assert_eq!(evaluate(&nodes, &edges), Err(EvaluationError::GraphCycle));
}

#[test]
fn disconnected_argument_node_is_rejected() {
    let (mut nodes, edges) = positive_case();
    nodes.push(node("orphan-context", NodeKind::Context));
    assert_eq!(evaluate(&nodes, &edges), Err(EvaluationError::OrphanNode));
}

#[test]
fn input_node_and_edge_permutation_preserves_exact_result() {
    let (nodes, edges) = positive_case();
    let expected = evaluate(&nodes, &edges).unwrap();
    let mut permuted_nodes = nodes;
    let mut permuted_edges = edges;
    permuted_nodes.reverse();
    permuted_edges.reverse();
    assert_eq!(
        evaluate(&permuted_nodes, &permuted_edges).unwrap(),
        expected
    );
}

#[test]
fn a_supported_claim_with_a_contradicted_state_is_still_blocked() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "claim-model-envelope")
        .unwrap()
        .claim
        .as_mut()
        .unwrap()
        .state = ClaimState::Contradicted;
    let result = evaluate(&nodes, &edges).unwrap();
    assert_eq!(
        result.status,
        AssuranceStatus::BlockedByUnsupportedOrStaleEvidence
    );
    assert!(blocker(&result, BlockerReason::ClaimContradicted));
}

#[test]
fn unknown_or_unbounded_case_scope_is_rejected_before_graph_evaluation() {
    let (nodes, edges) = positive_case();
    let mut frozen = policy();
    frozen.expected_scope.vertical_datum_id.clear();
    assert_eq!(
        evaluate_assurance_case(&nodes, &edges, "root", &frozen),
        Err(EvaluationError::InvalidScope)
    );
}

#[test]
fn node_validity_must_cover_the_entire_operation_window() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "evidence-model-validation")
        .unwrap()
        .valid_until = 99;
    assert_eq!(
        evaluate(&nodes, &edges),
        Err(EvaluationError::NodeValidityDoesNotCoverOperation)
    );
}

#[test]
fn authority_decision_must_remain_valid_through_operation_end() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.kind == NodeKind::AuthorityDecision)
        .unwrap()
        .authority_decision
        .as_mut()
        .unwrap()
        .valid_until = Some(99);
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(
        &result,
        BlockerReason::AuthorityDecisionOutstanding
    ));
    assert_eq!(
        result.outstanding_authority_roles,
        vec!["risk-acceptance".to_string()]
    );
}

#[test]
fn authority_decision_effective_after_operation_start_is_not_accepted_for_whole_window() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.kind == NodeKind::AuthorityDecision)
        .unwrap()
        .authority_decision
        .as_mut()
        .unwrap()
        .effective_at = 21;
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(
        &result,
        BlockerReason::AuthorityDecisionOutstanding
    ));
}

#[test]
fn unregistered_hazard_cannot_be_omitted_from_the_frozen_hazard_manifest() {
    let (mut nodes, mut edges) = positive_case();
    let mut extra = node("hazard-unregistered", NodeKind::Hazard);
    extra.hazard_id = Some("unregistered-hazard".into());
    nodes.push(extra);
    edges.push(edge(
        "context-operation",
        "hazard-unregistered",
        EdgeKind::Contextualizes,
    ));
    assert_eq!(
        evaluate(&nodes, &edges),
        Err(EvaluationError::HazardNotInRequiredManifest)
    );
}

#[test]
fn mitigation_without_hazard_attachment_is_blocked_even_if_it_has_evidence() {
    let (mut nodes, mut edges) = positive_case();
    let mut extra = node("mitigation-unattached", NodeKind::Mitigation);
    extra.mitigation_state = Some(ClaimState::Supported);
    nodes.push(extra);
    edges.push(edge(
        "evidence-mitigation-test",
        "mitigation-unattached",
        EdgeKind::Supports,
    ));
    assert!(blocker(
        &evaluate(&nodes, &edges).unwrap(),
        BlockerReason::MitigationNotAttachedToHazard
    ));
}

#[test]
fn defeater_must_challenge_a_claim_or_mitigation() {
    let (nodes, mut edges) = positive_case();
    edges.retain(|item| {
        !(item.from_node_id == "defeater-sensor-bias" && item.kind == EdgeKind::Challenges)
    });
    edges.push(edge(
        "defeater-sensor-bias",
        "authority-decision",
        EdgeKind::RequiresDecision,
    ));
    // The defeater remains attached to the case through an authority-decision edge,
    // but no Challenges edge identifies the claim or mitigation it is meant to challenge.
    assert!(blocker(
        &evaluate(&nodes, &edges).unwrap(),
        BlockerReason::DefeaterWithoutChallenge
    ));
}

#[test]
fn unknown_assumption_is_indeterminate_not_merely_incomplete() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "assumption-weather-profile")
        .unwrap()
        .assumption
        .as_mut()
        .unwrap()
        .state = AssumptionState::Unknown;
    assert_eq!(
        evaluate(&nodes, &edges).unwrap().status,
        AssuranceStatus::Indeterminate
    );
}

#[test]
fn indeterminate_mitigation_does_not_get_collapsed_to_unsupported() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "mitigation-conflict-screen")
        .unwrap()
        .mitigation_state = Some(ClaimState::Indeterminate);
    assert_eq!(
        evaluate(&nodes, &edges).unwrap().status,
        AssuranceStatus::Indeterminate
    );
}

#[test]
fn rejected_authority_decision_is_reported_as_outstanding() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.kind == NodeKind::AuthorityDecision)
        .unwrap()
        .authority_decision
        .as_mut()
        .unwrap()
        .status = AuthorityDecisionStatus::Rejected;
    let result = evaluate(&nodes, &edges).unwrap();
    assert_eq!(
        result.outstanding_authority_roles,
        vec!["risk-acceptance".to_string()]
    );
}

#[test]
fn hazard_must_be_explicitly_addressed_by_a_claim_or_strategy() {
    let (nodes, mut edges) = positive_case();
    edges.retain(|edge| edge.kind != EdgeKind::AddressesHazard);
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(&result, BlockerReason::HazardNotLinkedToArgument));
    assert_eq!(result.status, AssuranceStatus::IncompleteArgument);
}

#[test]
fn conflict_performance_receipt_cannot_substitute_for_model_credibility() {
    let (mut nodes, edges) = positive_case();
    let evidence = nodes
        .iter_mut()
        .find(|item| item.node_id == "evidence-model-validation")
        .unwrap()
        .evidence
        .as_mut()
        .unwrap();
    evidence.source_kind = EvidenceSourceKind::TrackR;
    evidence.purpose = EvidencePurpose::ConflictPerformance;
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(
        &result,
        BlockerReason::MissingRequiredEvidencePurpose
    ));
    assert!(blocker(
        &result,
        BlockerReason::InsufficientIndependentEvidenceGroups
    ));
}

#[test]
fn model_credibility_receipt_cannot_substitute_for_physical_mitigation_effectiveness() {
    let (mut nodes, edges) = positive_case();
    let evidence = nodes
        .iter_mut()
        .find(|item| item.node_id == "evidence-mitigation-test")
        .unwrap()
        .evidence
        .as_mut()
        .unwrap();
    evidence.source_kind = EvidenceSourceKind::TrackT;
    evidence.purpose = EvidencePurpose::ModelCredibility;
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(
        &result,
        BlockerReason::MissingRequiredEvidencePurpose
    ));
}

#[test]
fn source_owner_and_evidence_purpose_must_be_compatible() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "evidence-model-validation")
        .unwrap()
        .evidence
        .as_mut()
        .unwrap()
        .purpose = EvidencePurpose::ConflictPerformance;
    assert_eq!(
        evaluate(&nodes, &edges),
        Err(EvaluationError::EvidencePurposeSourceMismatch)
    );
}

#[test]
fn every_claim_requires_an_independent_policy_pinned_evidence_requirement() {
    let (nodes, edges) = positive_case();
    let mut frozen = policy();
    frozen
        .required_claim_evidence
        .retain(|item| item.claim_id != "claim-model-envelope");
    let result = evaluate_assurance_case(&nodes, &edges, "root", &frozen).unwrap();
    assert!(blocker(
        &result,
        BlockerReason::MissingClaimEvidenceRequirement
    ));
}

#[test]
fn every_mitigation_requires_an_independent_policy_pinned_evidence_requirement() {
    let (nodes, edges) = positive_case();
    let mut frozen = policy();
    frozen.required_mitigation_evidence.clear();
    let result = evaluate_assurance_case(&nodes, &edges, "root", &frozen);
    assert_eq!(result, Err(EvaluationError::InvalidPolicy));
}

#[test]
fn policy_requirement_for_absent_claim_is_reported_as_a_blocker() {
    let (nodes, edges) = positive_case();
    let mut frozen = policy();
    frozen
        .required_claim_evidence
        .push(ClaimEvidenceRequirement {
            claim_id: "claim-that-is-not-present".into(),
            required_purposes: vec![EvidencePurpose::ModelCredibility],
            minimum_independent_evidence_groups: 1,
        });
    let result = evaluate_assurance_case(&nodes, &edges, "root", &frozen).unwrap();
    assert!(blocker(
        &result,
        BlockerReason::EvidenceRequirementTargetMissing
    ));
}

#[test]
fn duplicated_evidence_purpose_in_policy_is_rejected() {
    let (nodes, edges) = positive_case();
    let mut frozen = policy();
    frozen.required_claim_evidence[1]
        .required_purposes
        .push(EvidencePurpose::ModelCredibility);
    assert_eq!(
        evaluate_assurance_case(&nodes, &edges, "root", &frozen),
        Err(EvaluationError::InvalidPolicy)
    );
}

#[test]
fn unrelated_evidence_purpose_does_not_increase_claim_independence_count() {
    let (mut nodes, mut edges) = positive_case();
    let mut extra = evidence_node(
        "evidence-extra-conflict-performance",
        EvidenceSourceKind::TrackR,
        EvidenceDisposition::Qualified,
        Some("independent-extra-performance-lab"),
    );
    extra.evidence.as_mut().unwrap().purpose = EvidencePurpose::ConflictPerformance;
    nodes.push(extra);
    edges.push(edge(
        "evidence-extra-conflict-performance",
        "claim-model-envelope",
        EdgeKind::Supports,
    ));
    let result = evaluate(&nodes, &edges).unwrap();
    assert_eq!(
        result
            .independent_evidence_groups_by_claim
            .iter()
            .find(|(claim_id, _)| claim_id == "claim-model-envelope")
            .unwrap()
            .1,
        1
    );
}

#[test]
fn owner_cannot_lower_claim_independence_below_frozen_policy_minimum() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "claim-model-envelope")
        .unwrap()
        .claim
        .as_mut()
        .unwrap()
        .minimum_independent_evidence_groups = 0;
    nodes
        .iter_mut()
        .find(|item| item.node_id == "evidence-model-validation")
        .unwrap()
        .evidence
        .as_mut()
        .unwrap()
        .correlation_group_id = None;
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(
        &result,
        BlockerReason::InsufficientIndependentEvidenceGroups
    ));
}

#[test]
fn mitigation_requires_policy_minimum_distinct_evidence_groups() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "evidence-mitigation-test")
        .unwrap()
        .evidence
        .as_mut()
        .unwrap()
        .correlation_group_id = None;
    let result = evaluate(&nodes, &edges).unwrap();
    assert!(blocker(
        &result,
        BlockerReason::MitigationInsufficientIndependentEvidenceGroups
    ));
}

#[test]
fn frozen_mitigation_minimum_cannot_be_zero() {
    let (nodes, edges) = positive_case();
    let mut frozen = policy();
    frozen.required_mitigation_evidence[0].minimum_independent_evidence_groups = 0;
    assert_eq!(
        evaluate_assurance_case(&nodes, &edges, "root", &frozen),
        Err(EvaluationError::InvalidPolicy)
    );
}

#[test]
fn mitigation_independence_count_is_reported_even_when_the_floor_fails() {
    let (mut nodes, edges) = positive_case();
    nodes
        .iter_mut()
        .find(|item| item.node_id == "evidence-mitigation-test")
        .unwrap()
        .evidence
        .as_mut()
        .unwrap()
        .correlation_group_id = None;
    let assessment = evaluate(&nodes, &edges).unwrap();
    assert_eq!(
        assessment.independent_evidence_groups_by_mitigation,
        vec![("mitigation-conflict-screen".to_string(), 0)]
    );
    assert!(blocker(
        &assessment,
        BlockerReason::MitigationInsufficientIndependentEvidenceGroups
    ));
}
