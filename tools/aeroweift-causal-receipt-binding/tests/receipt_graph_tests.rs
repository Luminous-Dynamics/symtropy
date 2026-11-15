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
        Stage::IntentConformance => ReceiptTime {
            evaluated_at: 6,
            produced_at: 7,
            valid_from: 0,
            valid_until: 100,
            observed_at: None,
            received_at: None,
        },
        Stage::ConflictScreening | Stage::Schedule => ReceiptTime {
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
        observation_traffic_class: None,
    }
}

fn source_observation_for_traffic(id: &str, sequence: u64, traffic_class: TrafficClass) -> Receipt {
    let mut result = receipt(
        id,
        Stage::SourceObservation,
        sequence,
        ClaimDisposition::Qualified,
        vec![],
        None,
    );
    result.observation_traffic_class = Some(traffic_class);
    result
}

fn surveillance_receipt(
    id: &str,
    sequence: u64,
    disposition: ClaimDisposition,
    traffic_class: TrafficClass,
    parents: Vec<ParentEdge>,
) -> Receipt {
    let mut result = receipt(
        id,
        Stage::SurveillanceAssessment,
        sequence,
        disposition,
        parents,
        None,
    );
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
    let cooperative_observation =
        source_observation_for_traffic("observation-cooperative-track", 8, Coop);
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
    let authorization = receipt("authorization", S::Authorization, 11, Q, vec![], None);
    let intent_conformance = receipt(
        "intent-conformance",
        S::IntentConformance,
        12,
        Q,
        vec![
            edge(&authorization, R::CurrentAuthorization),
            edge(&pnt, R::SubjectPosition),
            edge(&cooperative, R::SubjectTrack),
            edge(&restrictions, R::ActiveRestrictions),
        ],
        None,
    );
    let conflict = receipt(
        "conflict-screen",
        S::ConflictScreening,
        13,
        Q,
        vec![
            edge(&intent_conformance, R::OperationalIntentConformance),
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
        authorization,
        intent_conformance,
    ]
}

fn root_id() -> &'static str {
    "conflict-screen"
}

fn receipt_mut<'a>(receipts: &'a mut [Receipt], id: &str) -> &'a mut Receipt {
    receipts
        .iter_mut()
        .find(|receipt| receipt.receipt_id == id)
        .unwrap()
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
    assert_eq!(result.receipt_ids.len(), 13);
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
    receipt_mut(&mut receipts, "authorization")
        .subject
        .authorization_revision += 1;
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
    receipt_mut(&mut receipts, root_id())
        .parents
        .iter_mut()
        .find(|edge| edge.role == ParentRole::ReachableOccupancy)
        .unwrap()
        .parent_payload_sha256 = digest('f');
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ParentDigestMismatch)
    );
}

#[test]
fn missing_parent_is_rejected() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, root_id())
        .parents
        .iter_mut()
        .find(|edge| edge.role == ParentRole::ReachableOccupancy)
        .unwrap()
        .parent_receipt_id = "absent-parent".into();
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ParentMissing)
    );
}

#[test]
fn parent_stage_must_match_the_semantic_role() {
    let mut receipts = graph();
    let pnt = receipts[5].clone();
    *receipt_mut(&mut receipts, root_id())
        .parents
        .iter_mut()
        .find(|edge| edge.role == ParentRole::ReachableOccupancy)
        .unwrap() = edge(&pnt, ParentRole::ReachableOccupancy);
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::RoleMismatch)
    );
}

#[test]
fn repeated_parent_role_edge_is_rejected() {
    let mut receipts = graph();
    let duplicate_edge = receipt_mut(&mut receipts, root_id()).parents[0].clone();
    receipt_mut(&mut receipts, root_id())
        .parents
        .push(duplicate_edge);
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
    receipt_mut(&mut receipts, root_id())
        .parents
        .retain(|edge| {
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

#[test]
fn surveillance_receipt_cannot_be_relabelled_as_another_traffic_class() {
    let mut receipts = graph();
    let cooperative = receipts
        .iter_mut()
        .find(|receipt| receipt.receipt_id == "surveillance-cooperative")
        .unwrap();
    cooperative.traffic_class = Some(TrafficClass::NonCooperativeCrewed);
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::SurveillanceEvidenceMismatch)
    );
}

#[test]
fn every_required_traffic_class_has_exactly_one_occupancy_parent() {
    let mut receipts = graph();
    let profile = receipts
        .iter()
        .find(|receipt| receipt.receipt_id == "profile-policy")
        .unwrap()
        .clone();
    let observation = receipts
        .iter()
        .find(|receipt| receipt.receipt_id == "observation-cooperative-track")
        .unwrap()
        .clone();
    let duplicate = surveillance_receipt(
        "surveillance-cooperative-duplicate",
        12,
        ClaimDisposition::Qualified,
        TrafficClass::CooperativeCrewed,
        vec![
            edge(&profile, ParentRole::CoverageProfile),
            edge(&observation, ParentRole::SurveillanceObservation),
        ],
    );
    let duplicate_edge = edge(
        &duplicate,
        ParentRole::TrafficOccupancy(TrafficClass::CooperativeCrewed),
    );
    receipts
        .iter_mut()
        .find(|receipt| receipt.receipt_id == root_id())
        .unwrap()
        .parents
        .push(duplicate_edge);
    receipts.push(duplicate);
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::DuplicateParentRole)
    );
}

#[test]
fn qualified_surveillance_track_must_bind_its_observation() {
    let mut receipts = graph();
    let cooperative = receipts
        .iter_mut()
        .find(|receipt| receipt.receipt_id == "surveillance-cooperative")
        .unwrap();
    cooperative
        .parents
        .retain(|edge| edge.role != ParentRole::SurveillanceObservation);
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::SurveillanceEvidenceMismatch)
    );
}

#[test]
fn no_report_coverage_cannot_claim_an_observation_parent() {
    let mut receipts = graph();
    let observation = receipts
        .iter()
        .find(|receipt| receipt.receipt_id == "observation-cooperative-track")
        .unwrap()
        .clone();
    let noncooperative = receipts
        .iter_mut()
        .find(|receipt| receipt.receipt_id == "surveillance-noncooperative")
        .unwrap();
    noncooperative
        .parents
        .push(edge(&observation, ParentRole::SurveillanceObservation));
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::SurveillanceEvidenceMismatch)
    );
}

#[test]
fn conflict_screen_cannot_add_an_unrequired_traffic_class() {
    let mut receipts = graph();
    let profile = receipts
        .iter()
        .find(|receipt| receipt.receipt_id == "profile-policy")
        .unwrap()
        .clone();
    let extra_class = surveillance_receipt(
        "surveillance-uncrewed-extra",
        13,
        ClaimDisposition::Qualified,
        TrafficClass::ProfileQualifiedUncrewed,
        vec![edge(&profile, ParentRole::CoverageProfile)],
    );
    let extra_edge = edge(
        &extra_class,
        ParentRole::TrafficOccupancy(TrafficClass::ProfileQualifiedUncrewed),
    );
    receipts
        .iter_mut()
        .find(|receipt| receipt.receipt_id == root_id())
        .unwrap()
        .parents
        .push(extra_edge);
    receipts.push(extra_class);
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ProfileClassMismatch)
    );
}

#[test]
fn receipt_must_be_valid_at_its_own_evaluation_time() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, root_id()).time.valid_until = 8;
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ReceiptOutsideOwnValidity)
    );
}

#[test]
fn observation_must_arrive_before_its_evaluation() {
    let mut receipts = graph();
    let observation = receipt_mut(&mut receipts, "observation-position");
    observation.time.received_at = Some(3);
    observation.time.produced_at = 4;
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ObservationReceivedAfterEvaluation)
    );
}

#[test]
fn profile_class_order_permutation_preserves_semantic_result() {
    let mut receipts = graph();
    let expected = verify_graph(&receipts).unwrap();
    receipt_mut(&mut receipts, "profile-policy")
        .required_traffic_classes
        .as_mut()
        .unwrap()
        .reverse();
    assert_eq!(verify_graph(&receipts).unwrap(), expected);
}

#[test]
fn conflict_screening_requires_operational_intent_conformance() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, root_id())
        .parents
        .retain(|edge| edge.role != ParentRole::OperationalIntentConformance);
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::MissingRequiredParent)
    );
}

#[test]
fn nonconforming_intent_receipt_cannot_underwrite_conflict_screening() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "intent-conformance").disposition = ClaimDisposition::Nonconforming;
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ParentNotQualified)
    );
}

#[test]
fn revoked_authorization_cannot_underwrite_intent_conformance() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "authorization").disposition = ClaimDisposition::Revoked;
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::ParentNotQualified)
    );
}

#[test]
fn surveillance_observation_must_be_bound_to_the_same_traffic_class() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "observation-cooperative-track").observation_traffic_class =
        Some(TrafficClass::NonCooperativeCrewed);
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::SurveillanceEvidenceMismatch)
    );
}

#[test]
fn surveillance_observation_without_a_traffic_class_is_not_admissible() {
    let mut receipts = graph();
    receipt_mut(&mut receipts, "observation-cooperative-track").observation_traffic_class = None;
    assert_eq!(
        verify_graph(&receipts),
        Err(VerificationError::SurveillanceEvidenceMismatch)
    );
}
