use std::collections::BTreeMap;

use maritime_channel_conflict_contract::{
    ChannelConflictReport, ChannelConflictWorkload, ChannelDisposition, ChannelEdge, ChannelError,
    ChannelDirection, ChannelMovement, ChannelViolation, ConflictGraph, ConflictMember,
    ConflictRule, DirectionWindow, TopologyReviewState, evaluate_channel_conflicts,
};
use maritime_service_chain_contract::{ServiceUnit, TickWindow};
use maritime_service_chain_fleet::{
    DemandSelection, FleetObjective, FleetPlan, OperationPlacement,
};

fn operation(
    demand_id: &str,
    participant_id: &str,
    alternative_id: &str,
    segment_id: &str,
    start_tick: u64,
    end_tick: u64,
) -> OperationPlacement {
    OperationPlacement {
        demand_id: demand_id.to_owned(),
        participant_id: participant_id.to_owned(),
        alternative_id: alternative_id.to_owned(),
        segment_id: segment_id.to_owned(),
        start_tick,
        end_tick,
        resource_uses: Vec::new(),
    }
}

fn movement(
    movement_id: &str,
    demand_id: &str,
    segment_id: &str,
    edge_id: &str,
    direction_id: &str,
    start_tick: u64,
    end_tick: u64,
) -> ChannelMovement {
    ChannelMovement {
        movement_id: movement_id.to_owned(),
        operation: operation(
            demand_id,
            &format!("participant-{demand_id}"),
            "alt-a",
            segment_id,
            start_tick,
            end_tick,
        ),
        edge_id: edge_id.to_owned(),
        direction_id: direction_id.to_owned(),
        route_revision: "route-r1".to_owned(),
    }
}

fn plan_for(movements: &[ChannelMovement]) -> FleetPlan {
    let mut selections = BTreeMap::new();
    for item in movements {
        selections
            .entry(item.operation.demand_id.clone())
            .or_insert_with(|| DemandSelection {
                demand_id: item.operation.demand_id.clone(),
                participant_id: item.operation.participant_id.clone(),
                alternative_id: item.operation.alternative_id.clone(),
                delivery_tick: item.operation.end_tick,
                service_amount: 1,
                service_unit: ServiceUnit::Tonnes,
            });
    }
    FleetPlan {
        selections: selections.into_values().collect(),
        operations: movements
            .iter()
            .map(|item| item.operation.clone())
            .collect(),
        objective: FleetObjective {
            makespan_tick: movements
                .iter()
                .map(|item| item.operation.end_tick)
                .max()
                .unwrap_or(0),
            total_delivery_ticks: 0,
            total_idle_wait_ticks: 0,
            water_ml: 0,
            propulsion_wh: 0,
            auxiliary_wh: 0,
            shore_power_import_wh: 0,
            inland_transfer_wh: 0,
        },
    }
}

fn edge(
    segment_id: &str,
    edge_id: &str,
    from: &str,
    to: &str,
    windows: Vec<DirectionWindow>,
) -> ChannelEdge {
    let mut direction_ids = std::collections::BTreeSet::new();
    for item in &windows {
        for direction_id in &item.allowed_direction_ids {
            direction_ids.insert(direction_id.clone());
        }
    }
    let directions = direction_ids
        .into_iter()
        .map(|direction_id| ChannelDirection {
            direction_id,
            from_node_id: from.to_owned(),
            to_node_id: to.to_owned(),
        })
        .collect();
    edge_with_directions(segment_id, edge_id, from, to, directions, windows)
}

fn edge_with_directions(
    segment_id: &str,
    edge_id: &str,
    from: &str,
    to: &str,
    directions: Vec<ChannelDirection>,
    windows: Vec<DirectionWindow>,
) -> ChannelEdge {
    ChannelEdge {
        segment_id: segment_id.to_owned(),
        edge_id: edge_id.to_owned(),
        from_node_id: from.to_owned(),
        to_node_id: to.to_owned(),
        route_revision: "route-r1".to_owned(),
        directions,
        direction_windows: windows,
    }
}

fn direction(direction_id: &str, from: &str, to: &str) -> ChannelDirection {
    ChannelDirection {
        direction_id: direction_id.to_owned(),
        from_node_id: from.to_owned(),
        to_node_id: to.to_owned(),
    }
}

fn window(start: u64, end: u64, directions: &[&str]) -> DirectionWindow {
    DirectionWindow {
        window: TickWindow {
            start_tick: start,
            end_tick: end,
        },
        allowed_direction_ids: directions.iter().map(|item| (*item).to_owned()).collect(),
    }
}

fn member(edge_id: &str, direction_id: &str) -> ConflictMember {
    ConflictMember {
        edge_id: edge_id.to_owned(),
        direction_id: direction_id.to_owned(),
    }
}

fn rule(
    id: &str,
    zone: &str,
    member_a: ConflictMember,
    member_b: ConflictMember,
    minimum_headway_ticks: u64,
) -> ConflictRule {
    ConflictRule {
        rule_id: id.to_owned(),
        conflict_zone_id: zone.to_owned(),
        member_a,
        member_b,
        minimum_headway_ticks,
    }
}

fn graph(
    edges: Vec<ChannelEdge>,
    rules: Vec<ConflictRule>,
    topology_state: TopologyReviewState,
) -> ConflictGraph {
    ConflictGraph {
        graph_id: "synthetic-channel-graph".to_owned(),
        route_revision: "route-r1".to_owned(),
        time_basis_id: "synthetic-ticks-v1".to_owned(),
        topology_state,
        edges,
        conflict_rules: rules,
    }
}

fn evaluate(
    movements: Vec<ChannelMovement>,
    graph: ConflictGraph,
) -> Result<ChannelConflictReport, ChannelError> {
    let plan = plan_for(&movements);
    evaluate_channel_conflicts(&plan, &ChannelConflictWorkload { graph, movements })
}

fn one_way_main_channel(
    outbound_start: u64,
    minimum_headway_ticks: u64,
) -> (Vec<ChannelMovement>, ConflictGraph) {
    let movements = vec![
        movement(
            "move-in",
            "demand-in",
            "main-segment",
            "main-edge",
            "inbound",
            4,
            8,
        ),
        movement(
            "move-out",
            "demand-out",
            "main-segment",
            "main-edge",
            "outbound",
            outbound_start,
            outbound_start + 4,
        ),
    ];
    let graph = graph(
        vec![edge_with_directions(
            "main-segment",
            "main-edge",
            "outer",
            "inner",
            vec![
                direction("inbound", "outer", "inner"),
                direction("outbound", "inner", "outer"),
            ],
            vec![window(0, 10, &["inbound"]), window(10, 30, &["outbound"])],
        )],
        vec![rule(
            "opposing-channel",
            "one-way-main",
            member("main-edge", "inbound"),
            member("main-edge", "outbound"),
            minimum_headway_ticks,
        )],
        TopologyReviewState::Reviewed,
    );
    (movements, graph)
}

#[test]
fn one_way_channel_accepts_ordered_opposing_movements_after_required_headway() {
    let (movements, graph) = one_way_main_channel(11, 3);
    let report = evaluate(movements, graph).expect("valid graph and plan");
    assert_eq!(report.disposition, ChannelDisposition::ReadyForComparison);
    assert!(report.violations.is_empty());
    assert_eq!(report.movements_checked, 2);
    assert_eq!(report.conflict_pairs_checked, 1);
}

#[test]
fn opposing_one_way_movements_reject_insufficient_clearance_headway() {
    let (movements, graph) = one_way_main_channel(10, 3);
    let report = evaluate(movements, graph).expect("valid graph and plan");
    assert_eq!(report.disposition, ChannelDisposition::Infeasible);
    assert!(
        report
            .violations
            .contains(&ChannelViolation::ConflictHeadwayViolation {
                rule_id: "opposing-channel".to_owned(),
                first_movement_id: "move-in".to_owned(),
                second_movement_id: "move-out".to_owned(),
                overlaps: false,
                actual_gap_ticks: 2,
                required_gap_ticks: 3,
            })
    );
}

#[test]
fn merge_conflict_applies_headway_across_different_edges() {
    let movements = vec![
        movement(
            "from-west",
            "west-demand",
            "west-segment",
            "west-edge",
            "in",
            0,
            4,
        ),
        movement(
            "from-north",
            "north-demand",
            "north-segment",
            "north-edge",
            "in",
            5,
            8,
        ),
    ];
    let graph = graph(
        vec![
            edge(
                "west-segment",
                "west-edge",
                "west",
                "merge",
                vec![window(0, 20, &["in"])],
            ),
            edge(
                "north-segment",
                "north-edge",
                "north",
                "merge",
                vec![window(0, 20, &["in"])],
            ),
        ],
        vec![rule(
            "merge-headway",
            "merge-zone",
            member("west-edge", "in"),
            member("north-edge", "in"),
            2,
        )],
        TopologyReviewState::Reviewed,
    );
    let report = evaluate(movements, graph).expect("valid merge graph");
    assert_eq!(report.disposition, ChannelDisposition::Infeasible);
    assert!(
        report
            .violations
            .iter()
            .any(|item| matches!(item, ChannelViolation::ConflictHeadwayViolation { .. }))
    );
}

#[test]
fn crossing_conflict_rejects_overlapping_movements_on_distinct_edges() {
    let movements = vec![
        movement(
            "movement-a",
            "demand-a",
            "segment-a",
            "edge-a",
            "north",
            2,
            8,
        ),
        movement(
            "movement-b",
            "demand-b",
            "segment-b",
            "edge-b",
            "east",
            7,
            12,
        ),
    ];
    let graph = graph(
        vec![
            edge(
                "segment-a",
                "edge-a",
                "a0",
                "a1",
                vec![window(0, 20, &["north"])],
            ),
            edge(
                "segment-b",
                "edge-b",
                "b0",
                "b1",
                vec![window(0, 20, &["east"])],
            ),
        ],
        vec![rule(
            "crossing-rule",
            "crossing-zone",
            member("edge-a", "north"),
            member("edge-b", "east"),
            1,
        )],
        TopologyReviewState::Reviewed,
    );
    let report = evaluate(movements, graph).expect("valid crossing graph");
    assert_eq!(report.disposition, ChannelDisposition::Infeasible);
    assert!(
        report
            .violations
            .contains(&ChannelViolation::ConflictHeadwayViolation {
                rule_id: "crossing-rule".to_owned(),
                first_movement_id: "movement-a".to_owned(),
                second_movement_id: "movement-b".to_owned(),
                overlaps: true,
                actual_gap_ticks: 0,
                required_gap_ticks: 1,
            })
    );
}

#[test]
fn same_edge_half_open_handoff_is_allowed_when_headway_is_zero() {
    let movements = vec![
        movement("first", "demand-a", "segment", "edge", "forward", 0, 5),
        movement("second", "demand-b", "segment", "edge", "forward", 5, 8),
    ];
    let graph = graph(
        vec![edge(
            "segment",
            "edge",
            "start",
            "finish",
            vec![window(0, 20, &["forward"])],
        )],
        vec![rule(
            "same-lane",
            "single-lane",
            member("edge", "forward"),
            member("edge", "forward"),
            0,
        )],
        TopologyReviewState::Reviewed,
    );
    let report = evaluate(movements, graph).expect("valid half-open handoff");
    assert_eq!(report.disposition, ChannelDisposition::ReadyForComparison);
}

#[test]
fn one_way_policy_rejects_direction_not_open_in_the_operation_window() {
    let movements = vec![movement(
        "wrong-way",
        "demand-a",
        "segment",
        "edge",
        "outbound",
        2,
        4,
    )];
    let graph = graph(
        vec![edge_with_directions(
            "segment",
            "edge",
            "start",
            "finish",
            vec![
                direction("inbound", "start", "finish"),
                direction("outbound", "finish", "start"),
            ],
            vec![window(0, 20, &["inbound"])],
        )],
        Vec::new(),
        TopologyReviewState::Reviewed,
    );
    let report = evaluate(movements, graph).expect("valid direction policy");
    assert_eq!(report.disposition, ChannelDisposition::Infeasible);
    assert_eq!(
        report.violations,
        vec![ChannelViolation::DirectionNotAllowed {
            movement_id: "wrong-way".to_owned(),
            edge_id: "edge".to_owned(),
            direction_id: "outbound".to_owned(),
        }]
    );
}

#[test]
fn unknown_direction_identifier_is_a_structural_error() {
    let movements = vec![movement(
        "unknown-direction",
        "demand-a",
        "segment",
        "edge",
        "westbound",
        2,
        4,
    )];
    let graph = graph(
        vec![edge(
            "segment",
            "edge",
            "start",
            "finish",
            vec![window(0, 20, &["eastbound"])],
        )],
        Vec::new(),
        TopologyReviewState::Reviewed,
    );
    assert_eq!(
        evaluate(movements, graph),
        Err(ChannelError::UnknownMovementDirection(
            "westbound".to_owned()
        ))
    );
}

#[test]
fn direction_mapping_must_match_one_of_the_edge_endpoint_orientations() {
    let graph = graph(
        vec![edge_with_directions(
            "segment",
            "edge",
            "start",
            "finish",
            vec![direction("inbound", "other-start", "finish")],
            vec![window(0, 20, &["inbound"])],
        )],
        Vec::new(),
        TopologyReviewState::Reviewed,
    );
    let movements = vec![movement(
        "movement",
        "demand-a",
        "segment",
        "edge",
        "inbound",
        1,
        3,
    )];
    assert_eq!(
        evaluate(movements, graph),
        Err(ChannelError::InvalidEdge("edge".to_owned()))
    );
}

#[test]
fn direction_window_cannot_reference_an_undeclared_direction_mapping() {
    let graph = graph(
        vec![edge_with_directions(
            "segment",
            "edge",
            "start",
            "finish",
            vec![direction("inbound", "start", "finish")],
            vec![window(0, 20, &["outbound"])],
        )],
        Vec::new(),
        TopologyReviewState::Reviewed,
    );
    let movements = vec![movement(
        "movement",
        "demand-a",
        "segment",
        "edge",
        "inbound",
        1,
        3,
    )];
    assert_eq!(
        evaluate(movements, graph),
        Err(ChannelError::InvalidDirectionWindow("edge".to_owned()))
    );
}

#[test]
fn movement_not_fully_covered_by_a_direction_window_needs_evidence() {
    let movements = vec![movement(
        "movement", "demand-a", "segment", "edge", "inbound", 8, 12,
    )];
    let graph = graph(
        vec![edge(
            "segment",
            "edge",
            "start",
            "finish",
            vec![window(0, 10, &["inbound"]), window(10, 20, &["inbound"])],
        )],
        Vec::new(),
        TopologyReviewState::Reviewed,
    );
    let report = evaluate(movements, graph).expect("valid time windows");
    assert_eq!(report.disposition, ChannelDisposition::NeedsEvidence);
    assert_eq!(
        report.violations,
        vec![ChannelViolation::DirectionWindowMissing {
            movement_id: "movement".to_owned(),
            edge_id: "edge".to_owned(),
        }]
    );
}

#[test]
fn fleet_operation_on_declared_channel_segment_requires_a_movement_mapping() {
    let operation = operation("demand-a", "participant-demand-a", "alt-a", "segment", 1, 3);
    let plan = FleetPlan {
        selections: vec![DemandSelection {
            demand_id: "demand-a".to_owned(),
            participant_id: "participant-demand-a".to_owned(),
            alternative_id: "alt-a".to_owned(),
            delivery_tick: 3,
            service_amount: 1,
            service_unit: ServiceUnit::Tonnes,
        }],
        operations: vec![operation],
        objective: FleetObjective {
            makespan_tick: 3,
            total_delivery_ticks: 0,
            total_idle_wait_ticks: 0,
            water_ml: 0,
            propulsion_wh: 0,
            auxiliary_wh: 0,
            shore_power_import_wh: 0,
            inland_transfer_wh: 0,
        },
    };
    let graph = graph(
        vec![edge(
            "segment",
            "edge",
            "start",
            "finish",
            vec![window(0, 20, &["inbound"])],
        )],
        Vec::new(),
        TopologyReviewState::Reviewed,
    );
    let report = evaluate_channel_conflicts(
        &plan,
        &ChannelConflictWorkload {
            graph,
            movements: Vec::new(),
        },
    )
    .expect("missing mapping is a reportable coverage gap");
    assert_eq!(report.disposition, ChannelDisposition::NeedsEvidence);
    assert_eq!(
        report.violations,
        vec![ChannelViolation::MissingMovement {
            demand_id: "demand-a".to_owned(),
            segment_id: "segment".to_owned(),
        }]
    );
}

#[test]
fn channel_movement_must_be_exact_member_of_selected_fleet_plan() {
    let (mut movements, graph) = one_way_main_channel(11, 3);
    movements[0].operation.end_tick += 1;
    let plan = plan_for(&[movement(
        "selected",
        "demand-in",
        "main-segment",
        "main-edge",
        "inbound",
        4,
        8,
    )]);
    assert_eq!(
        evaluate_channel_conflicts(&plan, &ChannelConflictWorkload { graph, movements },),
        Err(ChannelError::FleetOperationNotFound(
            "main-segment".to_owned()
        ))
    );
}

#[test]
fn channel_movement_must_match_selected_alternative() {
    let movements = vec![movement(
        "movement", "demand-a", "segment", "edge", "in", 1, 3,
    )];
    let mut plan = plan_for(&movements);
    plan.selections[0].alternative_id = "other-alt".to_owned();
    let graph = graph(
        vec![edge(
            "segment",
            "edge",
            "start",
            "finish",
            vec![window(0, 20, &["in"])],
        )],
        Vec::new(),
        TopologyReviewState::Reviewed,
    );
    assert_eq!(
        evaluate_channel_conflicts(&plan, &ChannelConflictWorkload { graph, movements }),
        Err(ChannelError::FleetSelectionMismatch("demand-a".to_owned()))
    );
}

#[test]
fn topology_review_state_never_silently_promotes_unknown_or_unsupported_graph() {
    let movements = vec![movement(
        "movement", "demand-a", "segment", "edge", "in", 1, 3,
    )];
    let edges = vec![edge(
        "segment",
        "edge",
        "start",
        "finish",
        vec![window(0, 20, &["in"])],
    )];

    let unknown = evaluate(
        movements.clone(),
        graph(edges.clone(), Vec::new(), TopologyReviewState::Unknown),
    )
    .expect("unknown graph state");
    assert_eq!(unknown.disposition, ChannelDisposition::NeedsEvidence);

    let unsupported = evaluate(
        movements,
        graph(edges, Vec::new(), TopologyReviewState::Unsupported),
    )
    .expect("unsupported graph state");
    assert_eq!(unsupported.disposition, ChannelDisposition::Unsupported);
}

#[test]
fn equivalent_input_permutations_produce_identical_conflict_reports() {
    let (movements, graph) = one_way_main_channel(10, 3);
    let expected = evaluate(movements.clone(), graph.clone()).expect("valid graph");

    let mut permuted_movements = movements;
    permuted_movements.reverse();
    let permuted = evaluate(permuted_movements, graph).expect("permuted graph");
    assert_eq!(permuted, expected);
}

#[test]
fn unknown_conflict_edge_is_a_structural_error() {
    let graph = graph(
        vec![edge(
            "segment",
            "edge",
            "start",
            "finish",
            vec![window(0, 20, &["in"])],
        )],
        vec![rule(
            "bad-rule",
            "zone",
            member("edge", "in"),
            member("missing-edge", "out"),
            1,
        )],
        TopologyReviewState::Reviewed,
    );
    let movements = vec![movement(
        "movement", "demand-a", "segment", "edge", "in", 1, 3,
    )];
    assert_eq!(
        evaluate(movements, graph),
        Err(ChannelError::UnknownConflictEdge("missing-edge".to_owned()))
    );
}

fn reference_conflict(first: TickWindow, second: TickWindow, minimum_headway_ticks: u64) -> bool {
    let horizon = first.end_tick.max(second.end_tick);
    let mut simultaneous_ticks = 0_u64;
    for tick in 0..horizon {
        let first_active = first.start_tick <= tick && tick < first.end_tick;
        let second_active = second.start_tick <= tick && tick < second.end_tick;
        if first_active && second_active {
            simultaneous_ticks += 1;
        }
    }
    if simultaneous_ticks > 0 {
        return true;
    }

    let (earlier_end, later_start) = if first.start_tick <= second.start_tick {
        (first.end_tick, second.start_tick)
    } else {
        (second.end_tick, first.start_tick)
    };
    let mut free_ticks = 0_u64;
    for _ in earlier_end..later_start {
        free_ticks += 1;
    }
    free_ticks < minimum_headway_ticks
}

#[test]
fn production_channel_conflict_matches_independent_tick_oracle_for_frozen_interval_sweep() {
    let mut comparisons = 0;
    for first_start in 0..=6 {
        for first_duration in 1..=4 {
            for second_start in 0..=6 {
                for second_duration in 1..=4 {
                    for headway in 0..=3 {
                        let first = movement(
                            "first",
                            "demand-first",
                            "segment",
                            "edge",
                            "forward",
                            first_start,
                            first_start + first_duration,
                        );
                        let second = movement(
                            "second",
                            "demand-second",
                            "segment",
                            "edge",
                            "forward",
                            second_start,
                            second_start + second_duration,
                        );
                        let movements = vec![first, second];
                        let graph = graph(
                            vec![edge(
                                "segment",
                                "edge",
                                "start",
                                "finish",
                                vec![window(0, 20, &["forward"])],
                            )],
                            vec![rule(
                                "same-lane",
                                "lane",
                                member("edge", "forward"),
                                member("edge", "forward"),
                                headway,
                            )],
                            TopologyReviewState::Reviewed,
                        );
                        let report = evaluate(movements, graph).expect("valid synthetic fixture");
                        let expected = reference_conflict(
                            TickWindow {
                                start_tick: first_start,
                                end_tick: first_start + first_duration,
                            },
                            TickWindow {
                                start_tick: second_start,
                                end_tick: second_start + second_duration,
                            },
                            headway,
                        );
                        assert_eq!(
                            report.disposition == ChannelDisposition::Infeasible,
                            expected,
                            "interval mismatch first=({first_start},{first_duration}) second=({second_start},{second_duration}) headway={headway}"
                        );
                        comparisons += 1;
                    }
                }
            }
        }
    }
    assert_eq!(comparisons, 3136);
}
