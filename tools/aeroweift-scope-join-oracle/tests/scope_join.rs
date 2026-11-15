use aeroweift_scope_join_oracle::{
    CoverageRequirement, EvidenceIdentity, EvidenceOwner, JoinInput, OperationClaim,
    OperationScope, Reason, ScopeJoinDisposition, ScopeRequest, SurveillanceClaim,
    TimeWindow, TrafficClass, join_scope,
};

const OPERATION: OperationScope = OperationScope {
    evaluation_id: "EVAL-001",
    operation_id: "OP-001",
    flight_instance_id: "FLIGHT-001",
    aircraft_instance_id: "AIRCRAFT-001",
    intent_id: "INTENT-001",
    authorization_id: "AUTH-001",
    authorization_revision: 8,
};

const IDENTITY: EvidenceIdentity = EvidenceIdentity {
    profile_revision: "PROFILE-REV-3",
    time_scale_id: "TAI-SYNTHETIC",
    horizontal_frame_id: "LOCAL-ENU-1",
    vertical_datum_id: "DATUM-1",
};

fn window(start: i64, end: i64) -> TimeWindow {
    TimeWindow { start, end }
}

fn requirements() -> Vec<CoverageRequirement> {
    vec![
        CoverageRequirement {
            query_id: "Q-COOP-A",
            airspace_id: "AIRSPACE-1",
            cell_id: "CELL-A",
            traffic_class: TrafficClass::CooperativeCrewed,
            window: window(100, 200),
        },
        CoverageRequirement {
            query_id: "Q-NONCOOP-A",
            airspace_id: "AIRSPACE-1",
            cell_id: "CELL-A",
            traffic_class: TrafficClass::NonCooperativeCrewed,
            window: window(100, 200),
        },
    ]
}

fn operation_claim(owner: EvidenceOwner) -> OperationClaim {
    let validity_window = match owner {
        EvidenceOwner::Pnt => window(100, 101),
        EvidenceOwner::Reachability | EvidenceOwner::Intent => window(100, 200),
    };
    let receipt_ref = match owner {
        EvidenceOwner::Pnt => "PNT-RECEIPT-1",
        EvidenceOwner::Reachability => "REACH-RECEIPT-1",
        EvidenceOwner::Intent => "INTENT-RECEIPT-1",
    };
    let observed_at = match owner {
        EvidenceOwner::Intent => None,
        EvidenceOwner::Pnt | EvidenceOwner::Reachability => Some(99),
    };
    OperationClaim {
        owner,
        receipt_ref: Some(receipt_ref),
        scope: OPERATION,
        identity: IDENTITY,
        validity_window,
        evaluated_at: 100,
        observed_at,
        received_at: 100,
    }
}

fn surveillance_claim(requirement: CoverageRequirement, with_observation: bool) -> SurveillanceClaim {
    SurveillanceClaim {
        evaluation_id: OPERATION.evaluation_id,
        query_id: requirement.query_id,
        airspace_id: requirement.airspace_id,
        cell_id: requirement.cell_id,
        traffic_class: requirement.traffic_class,
        query_window: requirement.window,
        validity_window: window(90, 210),
        coverage_receipt_ref: Some("COVERAGE-RECEIPT-1"),
        dependency_snapshot_ref: Some("DEPENDENCY-SNAPSHOT-1"),
        observation_ref: if with_observation {
            Some("OBSERVATION-1")
        } else {
            None
        },
        observed_at: if with_observation { Some(99) } else { None },
        received_at: 100,
        identity: IDENTITY,
    }
}

fn valid_input() -> JoinInput {
    let required_coverage = requirements();
    JoinInput {
        request: ScopeRequest {
            operation: OPERATION,
            identity: IDENTITY,
            as_of_tick: 100,
            pnt_support_window: window(100, 101),
            operation_window: window(100, 200),
            required_coverage: required_coverage.clone(),
        },
        operation_claims: vec![
            operation_claim(EvidenceOwner::Pnt),
            operation_claim(EvidenceOwner::Reachability),
            operation_claim(EvidenceOwner::Intent),
        ],
        surveillance_claims: vec![
            surveillance_claim(required_coverage[0], true),
            surveillance_claim(required_coverage[1], false),
        ],
    }
}

fn assert_rejected(input: JoinInput, expected_reason: Reason) {
    let result = join_scope(&input);
    assert_eq!(result.disposition, ScopeJoinDisposition::NotAssessable);
    assert_eq!(result.reason, Some(expected_reason));
    assert!(result.matched_query_ids.is_empty());
    assert_eq!(result.source_snapshot, {
        let mut snapshot = input.clone();
        snapshot.request.required_coverage.sort_unstable();
        snapshot.operation_claims.sort_unstable();
        snapshot.surveillance_claims.sort_unstable();
        snapshot
    });
}

#[test]
fn exact_operation_authority_and_query_scope_matches() {
    let result = join_scope(&valid_input());
    assert_eq!(
        result.disposition,
        ScopeJoinDisposition::ScopeMatchedForComposition
    );
    assert_eq!(result.reason, None);
    assert_eq!(result.matched_query_ids, vec!["Q-COOP-A", "Q-NONCOOP-A"]);
}

#[test]
fn pnt_from_another_flight_instance_cannot_join() {
    let mut input = valid_input();
    input.operation_claims[0].scope.flight_instance_id = "FLIGHT-OTHER";
    assert_rejected(
        input,
        Reason::OperationScopeMismatch(EvidenceOwner::Pnt),
    );
}

#[test]
fn same_aircraft_with_a_different_flight_instance_is_not_the_same_subject() {
    let mut input = valid_input();
    input.operation_claims[1].scope.flight_instance_id = "FLIGHT-OTHER";
    assert_rejected(
        input,
        Reason::OperationScopeMismatch(EvidenceOwner::Reachability),
    );
}

#[test]
fn same_flight_with_a_different_intent_cannot_join() {
    let mut input = valid_input();
    input.operation_claims[2].scope.intent_id = "INTENT-OTHER";
    assert_rejected(
        input,
        Reason::OperationScopeMismatch(EvidenceOwner::Intent),
    );
}

#[test]
fn old_authorization_id_cannot_join_the_current_operation() {
    let mut input = valid_input();
    input.operation_claims[2].scope.authorization_id = "AUTH-OLD";
    assert_rejected(
        input,
        Reason::OperationScopeMismatch(EvidenceOwner::Intent),
    );
}

#[test]
fn superseded_authorization_revision_cannot_join() {
    let mut input = valid_input();
    input.operation_claims[2].scope.authorization_revision = 7;
    assert_rejected(
        input,
        Reason::OperationScopeMismatch(EvidenceOwner::Intent),
    );
}

#[test]
fn wrong_evaluation_identity_cannot_join() {
    let mut input = valid_input();
    input.operation_claims[0].scope.evaluation_id = "EVAL-OLD";
    assert_rejected(
        input,
        Reason::OperationScopeMismatch(EvidenceOwner::Pnt),
    );
}

#[test]
fn profile_frame_time_scale_or_datum_mismatch_blocks_join() {
    let mut input = valid_input();
    input.operation_claims[1].identity.horizontal_frame_id = "FRAME-OTHER";
    assert_rejected(
        input,
        Reason::OperationIdentityMismatch(EvidenceOwner::Reachability),
    );
}

#[test]
fn owner_claim_evaluated_after_decision_cutoff_is_rejected() {
    let mut input = valid_input();
    input.operation_claims[0].evaluated_at = 101;
    input.operation_claims[0].received_at = 101;
    assert_rejected(
        input,
        Reason::OwnerEvaluatedAfterCutoff(EvidenceOwner::Pnt),
    );
}

#[test]
fn late_receipt_cannot_leak_into_historical_cutoff() {
    let mut input = valid_input();
    input.operation_claims[0].observed_at = Some(95);
    input.operation_claims[0].received_at = 101;
    assert_rejected(
        input,
        Reason::OwnerReceiptAfterCutoff(EvidenceOwner::Pnt),
    );
}

#[test]
fn future_observation_cannot_be_used_before_it_occurs() {
    let mut input = valid_input();
    input.operation_claims[0].observed_at = Some(101);
    input.operation_claims[0].received_at = 100;
    assert_rejected(
        input,
        Reason::OwnerEventAfterCutoff(EvidenceOwner::Pnt),
    );
}

#[test]
fn event_time_after_first_receipt_is_rejected() {
    let mut input = valid_input();
    input.operation_claims[0].observed_at = Some(99);
    input.operation_claims[0].evaluated_at = 98;
    input.operation_claims[0].received_at = 98;
    assert_rejected(
        input,
        Reason::OwnerEventAfterReceipt(EvidenceOwner::Pnt),
    );
}

#[test]
fn owner_evaluation_after_receipt_is_rejected() {
    let mut input = valid_input();
    input.operation_claims[0].evaluated_at = 100;
    input.operation_claims[0].received_at = 99;
    input.operation_claims[0].observed_at = Some(99);
    assert_rejected(
        input,
        Reason::OwnerEvaluationAfterReceipt(EvidenceOwner::Pnt),
    );
}

#[test]
fn pnt_validity_must_cover_its_evaluation_support_epoch() {
    let mut input = valid_input();
    input.operation_claims[0].validity_window = window(90, 100);
    assert_rejected(
        input,
        Reason::OwnerValidityDoesNotCoverRequest(EvidenceOwner::Pnt),
    );
}

#[test]
fn reachability_validity_must_cover_the_requested_operation_window() {
    let mut input = valid_input();
    input.operation_claims[1].validity_window = window(100, 150);
    assert_rejected(
        input,
        Reason::OwnerValidityDoesNotCoverRequest(EvidenceOwner::Reachability),
    );
}

#[test]
fn intent_validity_must_cover_the_requested_operation_window() {
    let mut input = valid_input();
    input.operation_claims[2].validity_window = window(100, 150);
    assert_rejected(
        input,
        Reason::OwnerValidityDoesNotCoverRequest(EvidenceOwner::Intent),
    );
}

#[test]
fn missing_owner_receipt_blocks_the_join() {
    let mut input = valid_input();
    input.operation_claims[2].receipt_ref = None;
    assert_rejected(
        input,
        Reason::MissingOperationReceipt(EvidenceOwner::Intent),
    );
}

#[test]
fn missing_owner_claim_blocks_the_join() {
    let mut input = valid_input();
    input
        .operation_claims
        .retain(|claim| claim.owner != EvidenceOwner::Reachability);
    assert_rejected(
        input,
        Reason::MissingOperationClaim(EvidenceOwner::Reachability),
    );
}

#[test]
fn duplicate_owner_claim_is_not_silently_selected() {
    let mut input = valid_input();
    input
        .operation_claims
        .push(operation_claim(EvidenceOwner::Pnt));
    assert_rejected(
        input,
        Reason::DuplicateOperationClaim(EvidenceOwner::Pnt),
    );
}

#[test]
fn wrong_surveillance_cell_cannot_satisfy_a_required_query() {
    let mut input = valid_input();
    input.surveillance_claims[0].cell_id = "CELL-B";
    assert_rejected(input, Reason::SurveillanceQueryMismatch("Q-COOP-A"));
}

#[test]
fn cooperative_coverage_cannot_satisfy_noncooperative_query() {
    let mut input = valid_input();
    input.surveillance_claims[1].traffic_class = TrafficClass::CooperativeCrewed;
    assert_rejected(input, Reason::SurveillanceQueryMismatch("Q-NONCOOP-A"));
}

#[test]
fn surveillance_query_window_must_match_exactly() {
    let mut input = valid_input();
    input.surveillance_claims[0].query_window = window(100, 199);
    assert_rejected(input, Reason::SurveillanceQueryMismatch("Q-COOP-A"));
}

#[test]
fn coverage_validity_must_cover_the_entire_query_window() {
    let mut input = valid_input();
    input.surveillance_claims[0].validity_window = window(100, 150);
    assert_rejected(
        input,
        Reason::SurveillanceValidityDoesNotCoverQuery("Q-COOP-A"),
    );
}

#[test]
fn missing_coverage_receipt_blocks_the_query() {
    let mut input = valid_input();
    input.surveillance_claims[0].coverage_receipt_ref = None;
    assert_rejected(input, Reason::MissingCoverageReceipt("Q-COOP-A"));
}

#[test]
fn missing_dependency_snapshot_blocks_the_query() {
    let mut input = valid_input();
    input.surveillance_claims[1].dependency_snapshot_ref = None;
    assert_rejected(input, Reason::MissingDependencySnapshot("Q-NONCOOP-A"));
}

#[test]
fn surveillance_receipt_after_cutoff_is_rejected() {
    let mut input = valid_input();
    input.surveillance_claims[0].received_at = 101;
    assert_rejected(
        input,
        Reason::SurveillanceReceiptAfterCutoff("Q-COOP-A"),
    );
}

#[test]
fn surveillance_event_after_cutoff_is_rejected() {
    let mut input = valid_input();
    input.surveillance_claims[0].observed_at = Some(101);
    input.surveillance_claims[0].received_at = 100;
    assert_rejected(
        input,
        Reason::SurveillanceObservationAfterCutoff("Q-COOP-A"),
    );
}

#[test]
fn qualified_no_report_keeps_its_query_scope_without_inventing_an_observation() {
    let mut input = valid_input();
    input.surveillance_claims[1].observation_ref = None;
    input.surveillance_claims[1].observed_at = None;
    let result = join_scope(&input);
    assert_eq!(
        result.disposition,
        ScopeJoinDisposition::ScopeMatchedForComposition
    );
    assert_eq!(result.matched_query_ids.len(), 2);
    assert_eq!(input.surveillance_claims[1].observation_ref, None);
}

#[test]
fn observation_reference_and_event_time_must_be_present_together() {
    let mut input = valid_input();
    input.surveillance_claims[0].observed_at = None;
    assert_rejected(
        input,
        Reason::ObservationTimeReferenceMismatch("Q-COOP-A"),
    );
}

#[test]
fn missing_required_query_is_not_vacuously_accepted() {
    let mut input = valid_input();
    input.surveillance_claims.pop();
    assert_rejected(input, Reason::MissingSurveillanceClaim("Q-NONCOOP-A"));
}

#[test]
fn duplicate_required_query_id_is_rejected() {
    let mut input = valid_input();
    input
        .request
        .required_coverage
        .push(input.request.required_coverage[0]);
    assert_rejected(input, Reason::DuplicateCoverageRequirement);
}

#[test]
fn extra_unrequested_surveillance_claim_is_rejected() {
    let mut input = valid_input();
    let extra = input.surveillance_claims[0];
    input.surveillance_claims.push(SurveillanceClaim {
        query_id: "Q-EXTRA",
        ..extra
    });
    assert_rejected(input, Reason::UnexpectedSurveillanceClaim("Q-EXTRA"));
}

#[test]
fn input_order_permutation_preserves_the_complete_result() {
    let baseline = join_scope(&valid_input());
    let mut permuted = valid_input();
    permuted.request.required_coverage.reverse();
    permuted.operation_claims.reverse();
    permuted.surveillance_claims.reverse();
    assert_eq!(baseline, join_scope(&permuted));
}

#[test]
fn empty_authorization_or_flight_identity_is_not_admissible() {
    let mut input = valid_input();
    input.request.operation.authorization_id = "";
    assert_rejected(input, Reason::IncompleteOperationScope);
}

#[test]
fn empty_coverage_obligations_are_not_a_positive_control() {
    let mut input = valid_input();
    input.request.required_coverage.clear();
    assert_rejected(input, Reason::MissingRequiredCoverage);
}
