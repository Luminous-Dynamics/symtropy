use aeroweift_causal_receipt_binding::{
    ClaimDisposition, ParentEdge, ParentRole, Receipt, ReceiptTime, Stage, StructuralDisposition,
    SubjectScope, TimeWindow, TrafficClass, TrustFacet, VerificationError, VerificationPolicy,
    verify,
};

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn scope() -> SubjectScope {
    SubjectScope {
        scenario_id: "scenario-001".into(),
        scenario_manifest_sha256: digest('a'),
        operation_id: "mission-001".into(),
        aircraft_id: "aircraft-pseudo-01".into(),
        flight_id: "flight-001".into(),
        intent_id: "intent-001".into(),
        intent_revision: 7,
        intent_sha256: digest('b'),
        authorization_id: "authorization-001".into(),
        authorization_revision: 4,
        authorization_sha256: digest('c'),
        profile_id: "profile-fixed-wing".into(),
        profile_revision: "3".into(),
        time_scale_id: "UTC-SYNTHETIC".into(),
        horizontal_frame_id: "LOCAL-ENU-01".into(),
        vertical_datum_id: "SYNTHETIC-VD-01".into(),
        evaluation_window: TimeWindow { start: 0, end: 100 },
    }
}

fn policy() -> VerificationPolicy {
    VerificationPolicy {
        expected_subject: scope(),
        required_traffic_classes: vec![
            TrafficClass::CooperativeCrewed,
            TrafficClass::NonCooperativeCrewed,
        ],
        subject_traffic_class: TrafficClass::CooperativeCrewed,
    }
}

fn time(stage: Stage) -> ReceiptTime {
    match stage {
        Stage::SourceObservation => ReceiptTime {
            evaluated_at: 2,
            produced_at: 3,
            valid_from: 0,
            valid_until: 100,
            observed_at: Some(1),
            received_at: Some(2),
        },
        Stage::ClockCorrelation | Stage::ProfilePolicy | Stage::RestrictionSnapshot => {
            ReceiptTime {
                evaluated_at: 2,
                produced_at: 3,
                valid_from: 0,
                valid_until: 100,
                observed_at: None,
                received_at: None,
            }
        }
        Stage::PntQualification | Stage::SurveillanceAssessment => ReceiptTime {
            evaluated_at: 4,
            produced_at: 5,
            valid_from: 0,
            valid_until: 100,
            observed_at: None,
            received_at: None,
        },
        Stage::Reachability => ReceiptTime {
            evaluated_at: 6,
            produced_at: 7,
            valid_from: 0,
            valid_until: 100,
            observed_at: None,
            received_at: None,
        },
        Stage::ConflictScreening | Stage::Schedule | Stage::IntentConformance => ReceiptTime {
            evaluated_at: 8,
            produced_at: 9,
            valid_from: 0,
            valid_until: 100,
            observed_at: None,
            received_at: None,
        },
        Stage::Authorization => ReceiptTime {
            evaluated_at: 2,
            produced_at: 3,
            valid_from: 0,
            valid_until: 100,
            observed_at: None,
            received_at: None,
        },
    }
}

fn receipt(
    id: &str,
    stage: Stage,
    sequence: u64,
    disposition: ClaimDisposition,
    parents: Vec<ParentEdge>,
    required_traffic_classes: Option<Vec<TrafficClass>>,
) -> Receipt {
    let digest_char = if sequence < 10 {
        char::from_u32(0x30 + sequence as u32).unwrap()
    } else {
        char::from_u32(0x61 + (sequence as u32 % 6)).unwrap()
    };
    Receipt {
        receipt_id: id.into(),
        stage,
        schema_id: "aeroweift.receipt".into(),
        schema_revision: "0.1".into(),
        producer_id: format!("producer-{id}"),
        producer_revision: "rev-1".into(),
        producer_stream_id: "stream-main".into(),
        producer_sequence: sequence,
        payload_sha256: digest(digest_char),
        subject: scope(),
        disposition,
        time: time(stage),
        parents,
        required_traffic_classes,
        traffic_class: None,
    }
}

fn surveillance_receipt(
    id: &str,
    sequence: u64,
    disposition: ClaimDisposition,
    traffic_class: TrafficClass,
    parents: Vec<ParentEdge>,
) -> Receipt {
    let mut result = receipt(id, Stage::SurveillanceAssessment, sequence, disposition, parents, None);
    result.traffic_class = Some(traffic_class);
    result
}

fn edge(parent: &Receipt, role: ParentRole) -> ParentEdge {
    ParentEdge {
        parent_receipt_id: parent.receipt_id.clone(),
        parent_payload_sha256: parent.payload_sha256.clone(),
        role,
    }
}

fn graph() -> Vec<Receipt> {
    use ClaimDisposition::{NoReportQualifiedCoverage as NoReport, Qualified as Q};
    use ParentRole as R;
    use Stage as S;
    use TrafficClass::{CooperativeCrewed as Coop, NonCooperativeCrewed as NonCoop};

    let observation = receipt(
        "observation-position",
        S::SourceObservation,
        1,
        Q,
        vec![],
        None,
    );
    let sample_clock = receipt("clock-sample", S::ClockCorrelation, 2, Q, vec![], None);
    let assessment_clock = receipt("clock-assessment", S::ClockCorrelation, 3, Q, vec![], None);
    let profile = receipt(
        "profile-policy",
        S::ProfilePolicy,
        4,
        Q,
        vec![],
        Some(vec![Coop, NonCoop]),
    );
    let restrictions = receipt("restrictions", S::RestrictionSnapshot, 5, Q, vec![], None);
    let pnt = receipt(
        "pnt",
        S::PntQualification,
        6,
        Q,
        vec![
            edge(&observation, R::PositionObservation),
            edge(&sample_clock, R::SampleClockCorrelation),
            edge(&assessment_clock, R::AssessmentClockCorrelation),
        ],
        None,
    );
    let reachability = receipt(
        "reachability",
        S::Reachability,
        7,
        Q,
        vec![edge(&pnt, R::PntInitialPosition)],
        None,
    );
    let cooperative_observation = receipt(
        "observation-cooperative-track",
        S::SourceObservation,
        8,
        Q,
        vec![],
        None,
    );
    let cooperative = surveillance_receipt(
        "surveillance-cooperative",
        9,
        Q,
        Coop,
        vec![
            edge(&profile, R::CoverageProfile),
            edge(&cooperative_observation, R::SurveillanceObservation),
        ],
    );
    let noncooperative = surveillance_receipt(
        "surveillance-noncooperative",
        10,
        NoReport,
        NonCoop,
        vec![edge(&profile, R::CoverageProfile)],
    );
    let conflict = receipt(
        "conflict-screen",
        S::ConflictScreening,
        10,
        Q,
        vec![
            edge(&reachability, R::ReachableOccupancy),
            edge(&profile, R::RequiredTrafficClasses),
            edge(&restrictions, R::ActiveRestrictions),
            edge(&cooperative, R::TrafficOccupancy(Coop)),
            edge(&noncooperative, R::TrafficOccupancy(NonCoop)),
        ],
        None,
    );
    vec![
        observation,
        sample_clock,
        assessment_clock,
        profile,
        restrictions,
        pnt,
        reachability,
        cooperative,
        noncooperative,
        cooperative_observation,
        conflict,
    ]
}

fn root_id() -> &'static str {
    "conflict-screen"
}

fn verify_graph(
    receipts: &[Receipt],
) -> Result<aeroweift_causal_receipt_binding::VerificationResult, VerificationError> {
    verify(receipts, root_id(), &policy())
}

#[test]
fn positive_control_is_structurally_bound_only() {
    let result = verify_graph(&graph()).expect("positive control should pass");
    assert_eq!(result.disposition, StructuralDisposition::CausallyBoundOnly);
    assert_eq!(result.receipt_ids.len(), 11);
    assert!(result.receipt_ids.contains(&"conflict-screen".to_string()));
    assert_eq!(
        result.trust_not_evaluated,
        [
            TrustFacet::ByteIntegrity,
            TrustFacet::SignatureVerification,
            TrustFacet::IssuerAuthority,
            TrustFacet::RevocationEvaluation,
        ]
    );
}

#[test]
fn receipt_order_permutation_preserves_canonical_result() {
    let original = graph();
    let expected = verify_graph(&original).unwrap();
    let mut permuted = original;
    permuted.reverse();
    for receipt in &mut permuted {
        receipt.parents.reverse();
    }
    assert_eq!(verify_graph(&permuted).unwrap(), expected);
}

#[test]
fn cross_flight_splice_is_rejected() {
    let mut receipts = graph();
    receipts[0].subject.flight_id = "other-flight".into();
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::SubjectMismatch)
    );
}

#[test]
fn cross_aircraft_splice_is_rejected() {
    let mut receipts = graph();
    receipts[6].subject.aircraft_id = "other-aircraft".into();
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::SubjectMismatch)
    );
}

#[test]
fn authorization_revision_splice_is_rejected() {
    let mut receipts = graph();
    receipts[9].subject.authorization_revision += 1;
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::SubjectMismatch)
    );
}

#[test]
fn scenario_manifest_splice_is_rejected() {
    let mut receipts = graph();
    receipts[7].subject.scenario_manifest_sha256 = digest('e');
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::SubjectMismatch)
    );
}

#[test]
fn intent_digest_splice_is_rejected() {
    let mut receipts = graph();
    receipts[5].subject.intent_sha256 = digest('f');
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::SubjectMismatch)
    );
}

#[test]
fn evaluation_window_splice_is_rejected() {
    let mut receipts = graph();
    receipts[8].subject.evaluation_window.end = 99;
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::SubjectMismatch)
    );
}

#[test]
fn parent_payload_digest_must_match_exactly() {
    let mut receipts = graph();
    receipts[9].parents[0].parent_payload_sha256 = digest('f');
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ParentDigestMismatch)
    );
}

#[test]
fn missing_parent_is_rejected() {
    let mut receipts = graph();
    receipts[9].parents[0].parent_receipt_id = "absent-parent".into();
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ParentMissing)
    );
}

#[test]
fn parent_stage_must_match_the_semantic_role() {
    let mut receipts = graph();
    let pnt = receipts[5].clone();
    receipts[9].parents[0] = edge(&pnt, ParentRole::ReachableOccupancy);
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::RoleMismatch)
    );
}

#[test]
fn repeated_parent_role_edge_is_rejected() {
    let mut receipts = graph();
    receipts[9].parents.push(receipts[9].parents[0].clone());
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::DuplicateParentEdge)
    );
}

#[test]
fn missing_required_parent_edge_is_rejected() {
    let mut receipts = graph();
    receipts[6].parents.clear();
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::MissingRequiredParent)
    );
}

#[test]
fn unknown_parent_disposition_cannot_become_qualified() {
    let mut receipts = graph();
    receipts[5].disposition = ClaimDisposition::Stale;
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ParentNotQualified)
    );
}

#[test]
fn no_report_qualified_coverage_is_preserved_as_a_valid_occupancy_parent() {
    let receipts = graph();
    assert!(verify_graph(&receipts).is_ok());
    assert_eq!(
        receipts[8].disposition,
        ClaimDisposition::NoReportQualifiedCoverage
    );
}

#[test]
fn parent_produced_after_child_evaluation_is_rejected() {
    let mut receipts = graph();
    receipts[5].time.produced_at = 50;
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::CausalTimeViolation)
    );
}

#[test]
fn parent_validity_must_cover_child_evaluation() {
    let mut receipts = graph();
    receipts[5].time.valid_until = 6;
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::OutsideParentValidity)
    );
}

#[test]
fn future_observation_is_rejected_even_with_a_valid_wrapper() {
    let mut receipts = graph();
    receipts[0].time.observed_at = Some(8);
    receipts[0].time.received_at = Some(2);
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ObservationTimeInvalid)
    );
}

#[test]
fn observation_arrival_cannot_postdate_its_receipt() {
    let mut receipts = graph();
    receipts[0].time.received_at = Some(4);
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ReceiptTimeInversion)
    );
}

#[test]
fn observation_receipt_requires_observed_and_received_times() {
    let mut receipts = graph();
    receipts[0].time.observed_at = None;
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ObservationTimeInvalid)
    );
}

#[test]
fn duplicate_receipt_identifier_is_rejected() {
    let mut receipts = graph();
    receipts.push(receipts[0].clone());
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::DuplicateReceiptId)
    );
}

#[test]
fn producer_sequence_fork_is_rejected() {
    let mut receipts = graph();
    receipts[1].producer_id = receipts[0].producer_id.clone();
    receipts[1].producer_stream_id = receipts[0].producer_stream_id.clone();
    receipts[1].producer_sequence = receipts[0].producer_sequence;
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ForkedSequence)
    );
}

#[test]
fn unrelated_receipt_is_not_silently_included() {
    let mut receipts = graph();
    receipts.push(receipt(
        "orphan-source",
        Stage::SourceObservation,
        99,
        ClaimDisposition::Qualified,
        vec![],
        None,
    ));
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::OrphanReceipt)
    );
}

#[test]
fn profile_traffic_class_obligations_cannot_be_weakened() {
    let mut receipts = graph();
    receipts[3].required_traffic_classes = Some(vec![TrafficClass::CooperativeCrewed]);
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ProfileClassMismatch)
    );
}

#[test]
fn conflict_must_consume_each_profile_required_class() {
    let mut receipts = graph();
    receipts[9].parents.retain(|edge| {
        edge.role != ParentRole::TrafficOccupancy(TrafficClass::NonCooperativeCrewed)
    });
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::MissingRequiredParent)
    );
}

#[test]
fn duplicate_required_class_in_verifier_policy_is_rejected() {
    let mut invalid_policy = policy();
    invalid_policy
        .required_traffic_classes
        .push(TrafficClass::CooperativeCrewed);
    assert_eq!(
        verify(&graph(), root_id(), &invalid_policy),
        Err(VerificationError::ProfileClassMismatch)
    );
}

#[test]
fn missing_profile_required_class_set_is_rejected() {
    let mut receipts = graph();
    receipts[3].required_traffic_classes = None;
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ProfileClassMismatch)
    );
}

#[test]
fn malformed_payload_digest_is_rejected() {
    let mut receipts = graph();
    receipts[0].payload_sha256 = "not-a-digest".into();
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ReceiptMalformed)
    );
}

#[test]
fn incomplete_expected_subject_is_rejected() {
    let mut invalid_policy = policy();
    invalid_policy.expected_subject.vertical_datum_id.clear();
    assert_eq!(
        verify(&graph(), root_id(), &invalid_policy),
        Err(VerificationError::ExpectedScopeInvalid)
    );
}

#[test]
fn absent_root_is_rejected() {
    assert_eq!(
        verify(&graph(), "missing-root", &policy()),
        Err(VerificationError::RootMissing)
    );
}

#[test]
fn cycle_is_detected_even_when_causal_timestamps_tie() {
    let mut receipts = graph();
    let root = receipts
        .iter()
        .find(|receipt| receipt.receipt_id == root_id())
        .unwrap()
        .clone();
    let root_index = receipts
        .iter()
        .position(|receipt| receipt.receipt_id == root_id())
        .unwrap();
    receipts[root_index].time.evaluated_at = 9;
    receipts[root_index].time.produced_at = 9;
    receipts[root_index]
        .parents
        .push(edge(&root, ParentRole::PriorConflictAssessment));
    assert_eq!(verify_graph(&receipts), Err(VerificationError::Cycle));
}
