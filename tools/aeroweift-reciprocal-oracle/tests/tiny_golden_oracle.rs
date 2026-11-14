use aeroweift_reciprocal_oracle::{
    enumerate_joint_schedules, is_non_anticipative, joint_schedule_is_robust,
    option_externality_for_b, resource_claims_fit, robust_options_for_b, BranchDecision,
    ResourceCapacity, WorldBranch, RESOURCES, ROUTE_A_BYPASS, ROUTE_A_CROSSING,
    ROUTE_B_CROSSING,
};

const GOLDEN_FIXTURE: &str =
    include_str!("../../../fixtures/aeroweift-reciprocal-options/tiny-option-externality-v0.json");

#[test]
fn frozen_json_manifest_matches_the_oracle_fixture_identity() {
    assert!(GOLDEN_FIXTURE.contains("AW-RO-TRAP-001"));
    assert!(GOLDEN_FIXTURE.contains("reciprocal-option-preserving"));
    assert!(GOLDEN_FIXTURE.contains("clairvoyant-negative-control"));
    assert!(GOLDEN_FIXTURE.contains("\"delta\": 1"));
}

#[test]
fn greedy_crossing_allocation_removes_both_aircraft_joint_completion() {
    assert!(robust_options_for_b(&ROUTE_A_CROSSING, &RESOURCES).is_empty());
    assert!(!joint_schedule_is_robust(
        &ROUTE_A_CROSSING,
        &ROUTE_B_CROSSING,
        &RESOURCES
    ));
}

#[test]
fn bypass_counterfactual_restores_the_constrained_aircraft_option() {
    assert_eq!(
        robust_options_for_b(&ROUTE_A_BYPASS, &RESOURCES),
        vec!["route-b-crossing"]
    );
    assert!(joint_schedule_is_robust(
        &ROUTE_A_BYPASS,
        &ROUTE_B_CROSSING,
        &RESOURCES
    ));
    assert_eq!(
        option_externality_for_b(&ROUTE_A_CROSSING, &ROUTE_A_BYPASS, &RESOURCES),
        1
    );
}

#[test]
fn exhaustive_assignment_oracle_returns_only_the_reciprocal_schedule() {
    let schedules = enumerate_joint_schedules(&RESOURCES);
    assert_eq!(schedules.len(), 1);
    assert_eq!(schedules[0].aircraft_a_route, "route-a-bypass");
    assert_eq!(schedules[0].aircraft_b_route, "route-b-crossing");
}

#[test]
fn hidden_weather_branch_cannot_select_a_different_action() {
    let clairvoyant = [
        BranchDecision {
            world_branch: WorldBranch::Clear,
            observation_class: "initial-wind-unknown",
            route_id: "route-a-crossing",
        },
        BranchDecision {
            world_branch: WorldBranch::Gust,
            observation_class: "initial-wind-unknown",
            route_id: "route-a-bypass",
        },
    ];
    assert!(!is_non_anticipative(&clairvoyant));

    let observation_conditioned = [
        BranchDecision {
            world_branch: WorldBranch::Clear,
            observation_class: "initial-wind-unknown",
            route_id: "route-a-bypass",
        },
        BranchDecision {
            world_branch: WorldBranch::Gust,
            observation_class: "initial-wind-unknown",
            route_id: "route-a-bypass",
        },
    ];
    assert!(is_non_anticipative(&observation_conditioned));
}

#[test]
fn capacity_reduction_cannot_create_a_joint_schedule() {
    let reduced = [
        ResourceCapacity {
            resource_id: "shared-crossing",
            capacity: 0,
        },
        ResourceCapacity {
            resource_id: "wide-bypass",
            capacity: 1,
        },
        ResourceCapacity {
            resource_id: "destination-pad",
            capacity: 1,
        },
    ];
    assert!(enumerate_joint_schedules(&reduced).is_empty());
    assert!(enumerate_joint_schedules(&reduced).len() <= enumerate_joint_schedules(&RESOURCES).len());
}


#[test]
fn adjacent_half_open_claims_do_not_overlap_but_intersecting_claims_do() {
    use aeroweift_reciprocal_oracle::{RouteOption, ResourceClaim};

    const FIRST: [ResourceClaim; 1] = [ResourceClaim {
        resource_id: "shared-crossing",
        start_tick: 0,
        end_tick: 1,
    }];
    const ADJACENT: [ResourceClaim; 1] = [ResourceClaim {
        resource_id: "shared-crossing",
        start_tick: 1,
        end_tick: 2,
    }];
    const OVERLAPPING: [ResourceClaim; 1] = [ResourceClaim {
        resource_id: "shared-crossing",
        start_tick: 0,
        end_tick: 2,
    }];

    const ROUTE_FIRST: RouteOption = RouteOption {
        route_id: "first",
        aircraft_id: "aircraft-a",
        deadline_tick: 3,
        arrival_by_branch: [Some(1), Some(1)],
        claims: &FIRST,
    };
    const ROUTE_ADJACENT: RouteOption = RouteOption {
        route_id: "adjacent",
        aircraft_id: "aircraft-b",
        deadline_tick: 3,
        arrival_by_branch: [Some(2), Some(2)],
        claims: &ADJACENT,
    };
    const ROUTE_OVERLAPPING: RouteOption = RouteOption {
        route_id: "overlapping",
        aircraft_id: "aircraft-b",
        deadline_tick: 3,
        arrival_by_branch: [Some(2), Some(2)],
        claims: &OVERLAPPING,
    };

    let one_crossing = [ResourceCapacity {
        resource_id: "shared-crossing",
        capacity: 1,
    }];

    assert!(resource_claims_fit(
        &[&ROUTE_FIRST, &ROUTE_ADJACENT],
        &one_crossing
    ));
    assert!(!resource_claims_fit(
        &[&ROUTE_FIRST, &ROUTE_OVERLAPPING],
        &one_crossing
    ));
    assert_eq!(
        resource_claims_fit(&[&ROUTE_FIRST, &ROUTE_ADJACENT], &one_crossing),
        resource_claims_fit(&[&ROUTE_ADJACENT, &ROUTE_FIRST], &one_crossing)
    );
}

#[test]
fn unknown_resources_and_invalid_time_claims_fail_closed() {
    use aeroweift_reciprocal_oracle::{RouteOption, ResourceClaim};

    let unknown_resource = RouteOption {
        route_id: "unknown-resource",
        aircraft_id: "aircraft-a",
        deadline_tick: 28,
        arrival_by_branch: [Some(24), Some(26)],
        claims: &[ResourceClaim {
            resource_id: "undeclared-resource",
            start_tick: 1,
            end_tick: 2,
        }],
    };
    assert!(!resource_claims_fit(&[&unknown_resource], &RESOURCES));

    let invalid_interval = RouteOption {
        route_id: "invalid-interval",
        aircraft_id: "aircraft-a",
        deadline_tick: 28,
        arrival_by_branch: [Some(24), Some(26)],
        claims: &[ResourceClaim {
            resource_id: "wide-bypass",
            start_tick: 2,
            end_tick: 2,
        }],
    };
    assert!(!resource_claims_fit(&[&invalid_interval], &RESOURCES));
}
