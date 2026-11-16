use aeroweift_assurance_case::{
    AssuranceCasePolicy, AssuranceCaseScope, AssuranceEdge, AssuranceNode,
    AssuranceStatus, AuthorityDecisionData, AuthorityDecisionStatus, AuthorityRequirement,
    ClaimData, ClaimEvidenceRequirement, ClaimState, EdgeKind, EvidenceDisposition,
    EvidencePurpose, EvidenceReference, EvidenceSourceKind, MitigationEvidenceRequirement,
    NodeKind, evaluate_assurance_case,
};
use aeroweift_assurance_redteam::{
    AttackDisposition, AttackError, AttackMutation, AttackSpec, CampaignConfig,
    CaseSnapshot, OracleViolation, execute_attack, independent_oracle, run_campaign,
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

fn claim_node(id: &str, kind: NodeKind, minimum_groups: usize) -> AssuranceNode {
    let mut result = node(id, kind);
    result.claim = Some(ClaimData {
        state: ClaimState::Supported,
        minimum_independent_evidence_groups: minimum_groups,
    });
    result
}

fn evidence_node(
    id: &str,
    source_kind: EvidenceSourceKind,
    purpose: EvidencePurpose,
    correlation_group: &str,
) -> AssuranceNode {
    let mut result = node(id, NodeKind::Evidence);
    result.evidence = Some(EvidenceReference {
        source_receipt_id: format!("receipt-{id}"),
        source_payload_sha256: digest('2'),
        source_assessment_sha256: digest('3'),
        source_kind,
        purpose,
        disposition: EvidenceDisposition::Qualified,
        correlation_group_id: Some(correlation_group.into()),
        effective_cutoff: 50,
        knowledge_cutoff: 60,
    });
    result
}

fn authority_node() -> AssuranceNode {
    let mut result = node("authority-decision", NodeKind::AuthorityDecision);
    result.authority_decision = Some(AuthorityDecisionData {
        role: "risk-acceptance".into(),
        authority_id: "competent-authority".into(),
        status: AuthorityDecisionStatus::Accepted,
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

fn positive_case() -> CaseSnapshot {
    let nodes = vec![
        claim_node("root", NodeKind::TopClaim, 0),
        node("strategy", NodeKind::Strategy),
        claim_node("claim-model-envelope", NodeKind::Claim, 1),
        node("context-operation", NodeKind::Context),
        node("context-extra", NodeKind::Context),
        {
            let mut hazard = node("hazard-collision", NodeKind::Hazard);
            hazard.hazard_id = Some("collision-avoidance".into());
            hazard
        },
        {
            let mut mitigation = node("mitigation-conflict-screen", NodeKind::Mitigation);
            mitigation.mitigation_state = Some(ClaimState::Supported);
            mitigation
        },
        evidence_node(
            "evidence-model-validation",
            EvidenceSourceKind::TrackT,
            EvidencePurpose::ModelCredibility,
            "model-validation-lab-01",
        ),
        evidence_node(
            "evidence-mitigation-test",
            EvidenceSourceKind::ExternalFlightTest,
            EvidencePurpose::PhysicalMitigationEffectiveness,
            "independent-flight-test-lab-01",
        ),
        authority_node(),
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
        edge("context-extra", "root", EdgeKind::Contextualizes),
        edge("context-operation", "hazard-collision", EdgeKind::Contextualizes),
        edge("strategy", "hazard-collision", EdgeKind::AddressesHazard),
        edge(
            "mitigation-conflict-screen",
            "hazard-collision",
            EdgeKind::Mitigates,
        ),
        edge(
            "evidence-mitigation-test",
            "mitigation-conflict-screen",
            EdgeKind::Supports,
        ),
        edge(
            "hazard-collision",
            "authority-decision",
            EdgeKind::RequiresDecision,
        ),
    ];
    CaseSnapshot {
        policy: policy(),
        root_claim_id: "root".into(),
        nodes,
        edges,
    }
}

fn attack(id: &str, mutation: AttackMutation) -> AttackSpec {
    AttackSpec {
        attack_id: id.into(),
        mutation,
    }
}

#[test]
fn positive_case_is_review_ready_under_both_track_w_and_independent_oracle() {
    let case = positive_case();
    assert!(independent_oracle(&case).is_empty());
    let result = evaluate_assurance_case(
        &case.nodes,
        &case.edges,
        &case.root_claim_id,
        &case.policy,
    )
    .unwrap();
    assert_eq!(result.status, AssuranceStatus::ReviewReadyOnly);
    assert!(!result.permission_granted);
}

#[test]
fn dropped_claim_to_hazard_link_is_a_confirmed_counterexample() {
    let case = positive_case();
    let report = execute_attack(
        &case,
        &attack(
            "drop-hazard-argument-link",
            AttackMutation::RemoveEdge {
                from_node_id: "strategy".into(),
                to_node_id: "hazard-collision".into(),
                kind: EdgeKind::AddressesHazard,
            },
        ),
        42,
    )
    .unwrap();
    assert_eq!(
        report.disposition,
        AttackDisposition::ConfirmedCounterexample
    );
    assert_eq!(
        report.oracle_violations,
        vec![OracleViolation::HazardNotAddressedByArgument]
    );
    assert_ne!(
        report.verifier.status,
        Some(AssuranceStatus::ReviewReadyOnly)
    );
    assert!(report.minimized_witness.case.nodes.len() < case.nodes.len());
    assert_eq!(
        independent_oracle(&report.minimized_witness.case),
        [OracleViolation::HazardNotAddressedByArgument]
            .into_iter()
            .collect()
    );
}

#[test]
fn wrong_model_evidence_purpose_is_detected_independently() {
    let case = positive_case();
    let report = execute_attack(
        &case,
        &attack(
            "model-to-conflict-purpose-substitution",
            AttackMutation::SubstituteEvidencePurpose {
                node_id: "evidence-model-validation".into(),
                source_kind: EvidenceSourceKind::TrackR,
                purpose: EvidencePurpose::ConflictPerformance,
            },
        ),
        43,
    )
    .unwrap();
    assert_eq!(
        report.disposition,
        AttackDisposition::ConfirmedCounterexample
    );
    assert!(report.oracle_violations.contains(
        &OracleViolation::RequiredEvidencePurposeMissing
    ));
}

#[test]
fn foreign_aircraft_evidence_is_a_confirmed_scope_counterexample() {
    let case = positive_case();
    let report = execute_attack(
        &case,
        &attack(
            "splice-foreign-aircraft-evidence",
            AttackMutation::SpliceAircraftScope {
                node_id: "evidence-model-validation".into(),
                aircraft_id: "aircraft-pseudo-FOREIGN".into(),
            },
        ),
        44,
    )
    .unwrap();
    assert_eq!(
        report.disposition,
        AttackDisposition::ConfirmedCounterexample
    );
    assert!(report.oracle_violations.contains(&OracleViolation::ScopeSplice));
}

#[test]
fn rejected_authority_decision_is_detected_independently() {
    let case = positive_case();
    let report = execute_attack(
        &case,
        &attack(
            "reject-risk-acceptance",
            AttackMutation::ChangeAuthorityStatus {
                node_id: "authority-decision".into(),
                status: AuthorityDecisionStatus::Rejected,
            },
        ),
        45,
    )
    .unwrap();
    assert_eq!(
        report.disposition,
        AttackDisposition::ConfirmedCounterexample
    );
    assert!(report.oracle_violations.contains(
        &OracleViolation::AuthorityDecisionInvalid
    ));
}

#[test]
fn evidence_expiring_mid_operation_is_detected_independently() {
    let case = positive_case();
    let report = execute_attack(
        &case,
        &attack(
            "expire-model-evidence-before-operation-end",
            AttackMutation::ShortenNodeValidity {
                node_id: "evidence-model-validation".into(),
                valid_until: 99,
            },
        ),
        46,
    )
    .unwrap();
    assert_eq!(
        report.disposition,
        AttackDisposition::ConfirmedCounterexample
    );
    assert!(report.oracle_violations.contains(&OracleViolation::NodeValidityShort));
}

#[test]
fn clearing_correlation_identity_cannot_create_independence() {
    let case = positive_case();
    let report = execute_attack(
        &case,
        &attack(
            "remove-model-evidence-correlation",
            AttackMutation::RemoveCorrelationGroup {
                node_id: "evidence-model-validation".into(),
            },
        ),
        47,
    )
    .unwrap();
    assert_eq!(
        report.disposition,
        AttackDisposition::ConfirmedCounterexample
    );
    assert!(report.oracle_violations.contains(
        &OracleViolation::InsufficientIndependentEvidenceGroups
    ));
}

#[test]
fn context_only_mutation_can_return_no_counterexample_found_not_a_safety_pass() {
    let case = positive_case();
    let report = execute_attack(
        &case,
        &attack(
            "remove-non-normative-context-link",
            AttackMutation::RemoveEdge {
                from_node_id: "context-extra".into(),
                to_node_id: "root".into(),
                kind: EdgeKind::Contextualizes,
            },
        ),
        48,
    )
    .unwrap();
    assert_eq!(
        report.disposition,
        AttackDisposition::NoCounterexampleFoundWithinBudget
    );
    assert_eq!(
        report.verifier.status,
        Some(AssuranceStatus::ReviewReadyOnly)
    );
}

#[test]
fn nonexistent_attack_target_is_invalid_not_a_counterexample() {
    let case = positive_case();
    assert_eq!(
        execute_attack(
            &case,
            &attack(
                "remove-nonexistent-edge",
                AttackMutation::RemoveEdge {
                    from_node_id: "no-node".into(),
                    to_node_id: "no-target".into(),
                    kind: EdgeKind::AddressesHazard,
                },
            ),
            49,
        ),
        Err(AttackError::InvalidAttack)
    );
}

#[test]
fn baseline_must_be_positive_before_red_team_campaign() {
    let mut case = positive_case();
    case.edges.retain(|edge| edge.kind != EdgeKind::AddressesHazard);
    assert_eq!(
        run_campaign(
            &case,
            CampaignConfig {
                seed: 50,
                attack_budget: 5,
            }
        ),
        Err(AttackError::BaselineNotReviewReady)
    );
}

#[test]
fn seeded_campaign_is_deterministic_and_budget_bounded() {
    let case = positive_case();
    let config = CampaignConfig {
        seed: 0x5eed,
        attack_budget: 5,
    };
    let first = run_campaign(&case, config).unwrap();
    let second = run_campaign(&case, config).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.attacks.len(), 5);
    assert_eq!(first.candidate_count, 5);
}

#[test]
fn minimized_witness_replays_with_identical_oracle_violation_set() {
    let case = positive_case();
    let report = execute_attack(
        &case,
        &attack(
            "drop-hazard-argument-link",
            AttackMutation::RemoveEdge {
                from_node_id: "strategy".into(),
                to_node_id: "hazard-collision".into(),
                kind: EdgeKind::AddressesHazard,
            },
        ),
        51,
    )
    .unwrap();
    assert_eq!(
        independent_oracle(&report.minimized_witness.case),
        independent_oracle(&{
            let mut attacked = case.clone();
            attacked.edges.retain(|edge| {
                !(edge.from_node_id == "strategy"
                    && edge.to_node_id == "hazard-collision"
                    && edge.kind == EdgeKind::AddressesHazard)
            });
            attacked
        })
    );
    assert_eq!(
        report.minimized_witness.case.policy.policy_sha256,
        case.policy.policy_sha256
    );
}
