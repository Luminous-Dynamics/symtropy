use maritime_channel_conflict_contract::{
    ChannelMovement, ChannelSegmentRule, ChannelWorkload, DirectionPermission, DirectionWindow,
    EvidenceReceipt, EvidenceState, InteractionRelation, SegmentInteraction, TrafficDisposition,
    TrafficError, TrafficViolation, evaluate_channel_workload,
};
use maritime_service_chain_contract::TickWindow;
use maritime_service_chain_fleet::OperationPlacement;

fn evidence(id: &str, subject: &str, revision: &str) -> EvidenceReceipt {
    EvidenceReceipt {
        evidence_id: id.to_owned(),
        source_id: format!("source-{id}"),
        source_revision: "source-revision-1".to_owned(),
        subject_id: subject.to_owned(),
        subject_revision: revision.to_owned(),
        state: EvidenceState::Current,
        valid_from_tick: 0,
        valid_through_tick: 100,
        claim_ceiling: "synthetic channel topology fixture only".to_owned(),
    }
}

fn segment(
    id: &str,
    from: &str,
    to: &str,
    permission: DirectionPermission,
    same_segment_headway: u64,
) -> ChannelSegmentRule {
    ChannelSegmentRule {
        segment_id: id.to_owned(),
        from_node_id: from.to_owned(),
        to_node_id: to.to_owned(),
        route_revision: "route-r1".to_owned(),
        same_segment_min_separation_ticks: same_segment_headway,
        headway_evidence: evidence(&format!("headway-{id}"), id, "route-r1"),
        direction_windows: vec![DirectionWindow {
            window: TickWindow {
                start_tick: 0,
                end_tick: 100,
            },
            permission,
            evidence: evidence(&format!("direction-{id}"), id, "route-r1"),
        }],
    }
}

fn interaction(left: &str, right: &str, relation: InteractionRelation) -> SegmentInteraction {
    let (first, second) = if left <= right {
        (left, right)
    } else {
        (right, left)
    };
    SegmentInteraction {
        left_segment_id: left.to_owned(),
        right_segment_id: right.to_owned(),
        route_revision: "route-r1".to_owned(),
        relation,
        evidence: evidence(
            &format!("interaction-{first}-{second}"),
            &format!("{first}|{second}"),
            "route-r1",
        ),
    }
}

fn movement(
    id: &str,
    segment_id: &str,
    from: &str,
    to: &str,
    start: u64,
    end: u64,
) -> ChannelMovement {
    ChannelMovement {
        movement_id: id.to_owned(),
        segment_id: segment_id.to_owned(),
        route_revision: "route-r1".to_owned(),
        from_node_id: from.to_owned(),
        to_node_id: to.to_owned(),
        operation: OperationPlacement {
            demand_id: format!("demand-{id}"),
            participant_id: format!("participant-{id}"),
            alternative_id: "alternative-a".to_owned(),
            segment_id: segment_id.to_owned(),
            start_tick: start,
            end_tick: end,
            resource_uses: Vec::new(),
        },
    }
}

fn one_segment(movements: Vec<ChannelMovement>) -> ChannelWorkload {
    ChannelWorkload {
        corridor_id: "synthetic-corridor".to_owned(),
        time_basis_id: "synthetic-ticks-v1".to_owned(),
        route_revision: "route-r1".to_owned(),
        decision_tick: 0,
        segments: vec![segment(
            "segment-a",
            "node-a",
            "node-b",
            DirectionPermission::ForwardOnly,
            0,
        )],
        interactions: Vec::new(),
        movements,
    }
}

fn two_segments(
    relation: InteractionRelation,
    movements: Vec<ChannelMovement>,
) -> ChannelWorkload {
    ChannelWorkload {
        corridor_id: "synthetic-corridor".to_owned(),
        time_basis_id: "synthetic-ticks-v1".to_owned(),
        route_revision: "route-r1".to_owned(),
        decision_tick: 0,
        segments: vec![
            segment(
                "segment-a",
                "node-a",
                "node-b",
                DirectionPermission::ForwardOnly,
                0,
            ),
            segment(
                "segment-b",
                "node-c",
                "node-d",
                DirectionPermission::ForwardOnly,
                0,
            ),
        ],
        interactions: vec![interaction("segment-a", "segment-b", relation)],
        movements,
    }
}

#[test]
fn a_forward_movement_inside_its_declared_one_way_window_is_admitted() {
    let input = one_segment(vec![movement(
        "m-1",
        "segment-a",
        "node-a",
        "node-b",
        10,
        15,
    )]);
    let report = evaluate_channel_workload(&input).expect("valid channel workload");
    assert_eq!(report.disposition, TrafficDisposition::ReadyForComparison);
    assert!(report.violations.is_empty());
    assert_eq!(report.movements_checked, 1);
}

#[test]
fn reverse_movement_is_rejected_by_a_forward_only_channel_window() {
    let input = one_segment(vec![movement(
        "m-1",
        "segment-a",
        "node-b",
        "node-a",
        10,
        15,
    )]);
    let report = evaluate_channel_workload(&input).expect("valid channel workload");
    assert_eq!(report.disposition, TrafficDisposition::Infeasible);
    assert!(
        report
            .violations
            .contains(&TrafficViolation::MovementDirectionNotPermitted(
                "m-1".to_owned()
            ))
    );
}

#[test]
fn an_explicitly_closed_channel_window_rejects_movement() {
    let mut input = one_segment(vec![movement(
        "m-1",
        "segment-a",
        "node-a",
        "node-b",
        10,
        15,
    )]);
    input.segments[0].direction_windows[0].permission = DirectionPermission::Closed;
    let report = evaluate_channel_workload(&input).expect("valid closed window");
    assert_eq!(report.disposition, TrafficDisposition::Infeasible);
    assert!(
        report
            .violations
            .contains(&TrafficViolation::MovementOnClosedSegment(
                "m-1".to_owned()
            ))
    );
}

#[test]
fn a_gap_in_direction_windows_is_unknown_not_implicitly_open() {
    let mut input = one_segment(vec![movement(
        "m-1",
        "segment-a",
        "node-a",
        "node-b",
        30,
        35,
    )]);
    input.segments[0].direction_windows[0].window.end_tick = 20;
    let report = evaluate_channel_workload(&input).expect("valid but incomplete schedule");
    assert_eq!(report.disposition, TrafficDisposition::NeedsEvidence);
    assert!(
        report
            .violations
            .contains(&TrafficViolation::MissingDirectionCoverage(
                "segment-a".to_owned()
            ))
    );
}

#[test]
fn stale_direction_evidence_needs_review() {
    let mut input = one_segment(vec![movement(
        "m-1",
        "segment-a",
        "node-a",
        "node-b",
        10,
        15,
    )]);
    input.segments[0].direction_windows[0].evidence.state = EvidenceState::Stale;
    let report = evaluate_channel_workload(&input).expect("valid stale record");
    assert_eq!(report.disposition, TrafficDisposition::NeedsEvidence);
}

#[test]
fn revoked_direction_evidence_is_unsupported() {
    let mut input = one_segment(vec![movement(
        "m-1",
        "segment-a",
        "node-a",
        "node-b",
        10,
        15,
    )]);
    input.segments[0].direction_windows[0].evidence.state = EvidenceState::Revoked;
    let report = evaluate_channel_workload(&input).expect("valid revoked record");
    assert_eq!(report.disposition, TrafficDisposition::Unsupported);
}

#[test]
fn movement_route_revision_must_match_the_declared_route() {
    let mut input = one_segment(vec![movement(
        "m-1",
        "segment-a",
        "node-a",
        "node-b",
        10,
        15,
    )]);
    input.movements[0].route_revision = "route-old".to_owned();
    let report = evaluate_channel_workload(&input).expect("well-formed stale movement");
    assert_eq!(report.disposition, TrafficDisposition::NeedsEvidence);
    assert!(
        report
            .violations
            .contains(&TrafficViolation::SegmentRouteRevisionMismatch(
                "segment-a".to_owned()
            ))
    );
}

#[test]
fn stale_same_segment_headway_evidence_needs_review() {
    let mut input = one_segment(vec![movement(
        "m-1",
        "segment-a",
        "node-a",
        "node-b",
        10,
        15,
    )]);
    input.segments[0].headway_evidence.state = EvidenceState::Stale;
    let report = evaluate_channel_workload(&input).expect("valid stale headway");
    assert_eq!(report.disposition, TrafficDisposition::NeedsEvidence);
}

#[test]
fn segment_rule_from_an_old_route_revision_needs_evidence() {
    let mut input = one_segment(vec![movement(
        "m-1",
        "segment-a",
        "node-a",
        "node-b",
        10,
        15,
    )]);
    input.segments[0].route_revision = "route-old".to_owned();
    input.segments[0].headway_evidence.subject_revision = "route-old".to_owned();
    input.segments[0].direction_windows[0]
        .evidence
        .subject_revision = "route-old".to_owned();
    let report = evaluate_channel_workload(&input).expect("well-formed old segment profile");
    assert_eq!(report.disposition, TrafficDisposition::NeedsEvidence);
    assert!(
        report
            .violations
            .contains(&TrafficViolation::SegmentRouteRevisionMismatch(
                "segment-a".to_owned()
            ))
    );
}

#[test]
fn overlapping_movements_on_one_segment_conflict() {
    let mut input = one_segment(vec![
        movement("m-1", "segment-a", "node-a", "node-b", 10, 20),
        movement("m-2", "segment-a", "node-a", "node-b", 15, 25),
    ]);
    input.segments[0].same_segment_min_separation_ticks = 0;
    let report = evaluate_channel_workload(&input).expect("valid pair");
    assert_eq!(report.disposition, TrafficDisposition::Infeasible);
    assert!(report.violations.iter().any(|item| matches!(
        item,
        TrafficViolation::SameSegmentHeadwayConflict { .. }
    )));
}

#[test]
fn same_segment_headway_must_meet_the_declared_minimum() {
    let mut input = one_segment(vec![
        movement("m-1", "segment-a", "node-a", "node-b", 10, 20),
        movement("m-2", "segment-a", "node-a", "node-b", 22, 30),
    ]);
    input.segments[0].same_segment_min_separation_ticks = 3;
    let report = evaluate_channel_workload(&input).expect("valid pair");
    assert_eq!(report.disposition, TrafficDisposition::Infeasible);
}

#[test]
fn same_segment_exact_minimum_headway_is_accepted() {
    let mut input = one_segment(vec![
        movement("m-1", "segment-a", "node-a", "node-b", 10, 20),
        movement("m-2", "segment-a", "node-a", "node-b", 23, 30),
    ]);
    input.segments[0].same_segment_min_separation_ticks = 3;
    let report = evaluate_channel_workload(&input).expect("valid pair");
    assert_eq!(report.disposition, TrafficDisposition::ReadyForComparison);
}

#[test]
fn same_segment_half_open_back_to_back_handoff_is_allowed_at_zero_headway() {
    let input = one_segment(vec![
        movement("m-1", "segment-a", "node-a", "node-b", 10, 20),
        movement("m-2", "segment-a", "node-a", "node-b", 20, 30),
    ]);
    let report = evaluate_channel_workload(&input).expect("valid handoff");
    assert_eq!(report.disposition, TrafficDisposition::ReadyForComparison);
}

#[test]
fn cross_segment_overlap_is_rejected_when_the_pair_shares_a_conflict_zone() {
    let input = two_segments(
        InteractionRelation::Conflict {
            minimum_separation_ticks: 0,
        },
        vec![
            movement("m-a", "segment-a", "node-a", "node-b", 10, 20),
            movement("m-b", "segment-b", "node-c", "node-d", 15, 25),
        ],
    );
    let report = evaluate_channel_workload(&input).expect("valid conflict graph");
    assert_eq!(report.disposition, TrafficDisposition::Infeasible);
    assert!(report.violations.iter().any(|item| matches!(
        item,
        TrafficViolation::CrossSegmentConflict { .. }
    )));
}

#[test]
fn cross_segment_minimum_clearance_is_enforced() {
    let input = two_segments(
        InteractionRelation::Conflict {
            minimum_separation_ticks: 4,
        },
        vec![
            movement("m-a", "segment-a", "node-a", "node-b", 10, 20),
            movement("m-b", "segment-b", "node-c", "node-d", 23, 30),
        ],
    );
    let report = evaluate_channel_workload(&input).expect("valid conflict graph");
    assert_eq!(report.disposition, TrafficDisposition::Infeasible);
}

#[test]
fn cross_segment_exact_clearance_is_accepted() {
    let input = two_segments(
        InteractionRelation::Conflict {
            minimum_separation_ticks: 4,
        },
        vec![
            movement("m-a", "segment-a", "node-a", "node-b", 10, 20),
            movement("m-b", "segment-b", "node-c", "node-d", 24, 30),
        ],
    );
    let report = evaluate_channel_workload(&input).expect("valid conflict graph");
    assert_eq!(report.disposition, TrafficDisposition::ReadyForComparison);
}

#[test]
fn a_reviewed_independent_segment_pair_can_overlap() {
    let input = two_segments(
        InteractionRelation::Independent,
        vec![
            movement("m-a", "segment-a", "node-a", "node-b", 10, 20),
            movement("m-b", "segment-b", "node-c", "node-d", 10, 20),
        ],
    );
    let report = evaluate_channel_workload(&input).expect("explicit independence");
    assert_eq!(report.disposition, TrafficDisposition::ReadyForComparison);
}

#[test]
fn a_missing_segment_pair_declaration_blocks_admission() {
    let mut input = two_segments(
        InteractionRelation::Independent,
        vec![
            movement("m-a", "segment-a", "node-a", "node-b", 10, 20),
            movement("m-b", "segment-b", "node-c", "node-d", 10, 20),
        ],
    );
    input.interactions.clear();
    let report = evaluate_channel_workload(&input).expect("incomplete interaction matrix");
    assert_eq!(report.disposition, TrafficDisposition::NeedsEvidence);
    assert!(
        report
            .violations
            .contains(&TrafficViolation::MissingSegmentInteraction {
                left_segment_id: "segment-a".to_owned(),
                right_segment_id: "segment-b".to_owned(),
            })
    );
}

#[test]
fn stale_pair_interaction_evidence_needs_review() {
    let mut input = two_segments(
        InteractionRelation::Independent,
        vec![
            movement("m-a", "segment-a", "node-a", "node-b", 10, 20),
            movement("m-b", "segment-b", "node-c", "node-d", 10, 20),
        ],
    );
    input.interactions[0].evidence.state = EvidenceState::Stale;
    let report = evaluate_channel_workload(&input).expect("valid stale interaction");
    assert_eq!(report.disposition, TrafficDisposition::NeedsEvidence);
}

#[test]
fn pair_interaction_evidence_must_cover_both_movement_windows() {
    let mut input = two_segments(
        InteractionRelation::Independent,
        vec![
            movement("m-a", "segment-a", "node-a", "node-b", 10, 20),
            movement("m-b", "segment-b", "node-c", "node-d", 30, 40),
        ],
    );
    input.interactions[0].evidence.valid_through_tick = 25;
    let report = evaluate_channel_workload(&input).expect("valid but stale evidence interval");
    assert_eq!(report.disposition, TrafficDisposition::NeedsEvidence);
}

#[test]
fn evidence_subject_must_bind_to_exact_segment() {
    let mut input = one_segment(vec![movement(
        "m-1",
        "segment-a",
        "node-a",
        "node-b",
        10,
        15,
    )]);
    input.segments[0].direction_windows[0].evidence.subject_id = "other-segment".to_owned();
    assert_eq!(
        evaluate_channel_workload(&input),
        Err(TrafficError::InvalidEvidenceReceipt(
            "direction-segment-a".to_owned()
        ))
    );
}

#[test]
fn pair_evidence_subject_is_canonical_even_if_pair_order_is_reversed() {
    let mut input = two_segments(
        InteractionRelation::Independent,
        vec![
            movement("m-a", "segment-a", "node-a", "node-b", 10, 20),
            movement("m-b", "segment-b", "node-c", "node-d", 10, 20),
        ],
    );
    input.interactions[0].left_segment_id = "segment-b".to_owned();
    input.interactions[0].right_segment_id = "segment-a".to_owned();
    let report = evaluate_channel_workload(&input).expect("canonical pair evidence");
    assert_eq!(report.disposition, TrafficDisposition::ReadyForComparison);
}

#[test]
fn duplicate_movement_identity_is_a_structural_error() {
    let first = movement("m-1", "segment-a", "node-a", "node-b", 10, 15);
    let mut second = first.clone();
    second.operation.start_tick = 20;
    second.operation.end_tick = 25;
    let input = one_segment(vec![first, second]);
    assert_eq!(
        evaluate_channel_workload(&input),
        Err(TrafficError::DuplicateMovement("m-1".to_owned()))
    );
}

#[test]
fn movement_endpoints_must_match_declared_segment_endpoints() {
    let input = one_segment(vec![movement(
        "m-1",
        "segment-a",
        "node-a",
        "node-c",
        10,
        15,
    )]);
    assert_eq!(
        evaluate_channel_workload(&input),
        Err(TrafficError::InvalidMovement("m-1".to_owned()))
    );
}

#[test]
fn equivalent_input_permutations_produce_identical_reports() {
    let first = movement("m-a", "segment-a", "node-a", "node-b", 10, 20);
    let second = movement("m-b", "segment-b", "node-c", "node-d", 24, 30);
    let mut original = two_segments(
        InteractionRelation::Conflict {
            minimum_separation_ticks: 4,
        },
        vec![first.clone(), second.clone()],
    );
    let expected = evaluate_channel_workload(&original).expect("valid workload");
    original.movements.reverse();
    original.segments.reverse();
    original.interactions.reverse();
    original.segments[0].direction_windows.reverse();
    assert_eq!(
        evaluate_channel_workload(&original).expect("permuted workload"),
        expected
    );
}
