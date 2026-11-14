//! Exhaustive, fixture-specific reference oracle for AeroWeft reciprocal options.
//!
//! This is a tiny finite oracle, not a production scheduler, aircraft model,
//! reachability solver, policy-tree engine, or operational aviation component.
//! It enumerates the hand-solvable two-aircraft fixture tracked by Symtropy
//! #5207. Its purpose is to make the claimed counterexample falsifiable and
//! to provide a separately implemented answer for later adapters.
//!
//! All times, resources, routes, capacities, and aircraft are fictional.
//! Every selected route must be feasible in every declared world branch.
//! Shared resources are checked using half-open integer-tick intervals.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Declared synthetic weather branches in the golden fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldBranch {
    /// Synthetic clear-weather branch.
    Clear,
    /// Synthetic gust branch.
    Gust,
}

/// One half-open claim on a synthetic capacity resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceClaim {
    /// Stable resource identifier.
    pub resource_id: &'static str,
    /// Inclusive start tick.
    pub start_tick: u16,
    /// Exclusive end tick.
    pub end_tick: u16,
}

/// Capacity of one synthetic resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceCapacity {
    /// Stable resource identifier.
    pub resource_id: &'static str,
    /// Maximum simultaneous claim count.
    pub capacity: u8,
}

/// One route option in the frozen two-aircraft example.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteOption {
    /// Stable route identifier.
    pub route_id: &'static str,
    /// Aircraft whose frozen decision domain contains this route.
    pub aircraft_id: &'static str,
    /// Latest permitted synthetic arrival tick.
    pub deadline_tick: u16,
    /// Arrival tick in clear-then-gust order. None means unsupported.
    pub arrival_by_branch: [Option<u16>; 2],
    /// Resources consumed by this route.
    pub claims: &'static [ResourceClaim],
}

/// A complete allocation for the two aircraft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JointSchedule {
    /// Aircraft A's selected route.
    pub aircraft_a_route: &'static str,
    /// Aircraft B's selected route.
    pub aircraft_b_route: &'static str,
}

/// A decision delivered under one observation-equivalence class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchDecision {
    /// Synthetic world branch used by the evaluator.
    pub world_branch: WorldBranch,
    /// Observation-equivalence class actually delivered to the policy.
    pub observation_class: &'static str,
    /// Action selected by the policy.
    pub route_id: &'static str,
}

/// Synthetic route A through the shared crossing.
pub const ROUTE_A_CROSSING: RouteOption = RouteOption {
    route_id: "route-a-crossing",
    aircraft_id: "aircraft-a",
    deadline_tick: 28,
    arrival_by_branch: [Some(20), Some(20)],
    claims: &[
        ResourceClaim {
            resource_id: "shared-crossing",
            start_tick: 10,
            end_tick: 12,
        },
        ResourceClaim {
            resource_id: "destination-pad",
            start_tick: 20,
            end_tick: 21,
        },
    ],
};

/// Synthetic route A using the wider, longer bypass.
pub const ROUTE_A_BYPASS: RouteOption = RouteOption {
    route_id: "route-a-bypass",
    aircraft_id: "aircraft-a",
    deadline_tick: 28,
    arrival_by_branch: [Some(24), Some(26)],
    claims: &[
        ResourceClaim {
            resource_id: "wide-bypass",
            start_tick: 12,
            end_tick: 18,
        },
        ResourceClaim {
            resource_id: "destination-pad",
            start_tick: 24,
            end_tick: 27,
        },
    ],
};

/// Synthetic constrained-aircraft route B through the shared crossing.
pub const ROUTE_B_CROSSING: RouteOption = RouteOption {
    route_id: "route-b-crossing",
    aircraft_id: "aircraft-b",
    deadline_tick: 23,
    arrival_by_branch: [Some(22), Some(22)],
    claims: &[
        ResourceClaim {
            resource_id: "shared-crossing",
            start_tick: 10,
            end_tick: 12,
        },
        ResourceClaim {
            resource_id: "destination-pad",
            start_tick: 22,
            end_tick: 23,
        },
    ],
};

/// Frozen resource capacities in the golden fixture.
pub const RESOURCES: [ResourceCapacity; 3] = [
    ResourceCapacity {
        resource_id: "shared-crossing",
        capacity: 1,
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

const ROUTES_A: [RouteOption; 2] = [ROUTE_A_CROSSING, ROUTE_A_BYPASS];
const ROUTES_B: [RouteOption; 1] = [ROUTE_B_CROSSING];

fn resource_capacity(
    resource_id: &str,
    capacities: &[ResourceCapacity],
) -> Option<u8> {
    capacities
        .iter()
        .find(|resource| resource.resource_id == resource_id)
        .map(|resource| resource.capacity)
}

/// Whether a route is supported and meets its deadline in every declared branch.
pub fn route_is_robust(route: &RouteOption) -> bool {
    route
        .arrival_by_branch
        .iter()
        .all(|arrival| arrival.is_some_and(|tick| tick <= route.deadline_tick))
}

/// Check all selected half-open resource claims against finite capacities.
///
/// The checker partitions time at every claim endpoint. On each non-empty
/// elementary interval it counts claims covering that complete interval.
/// Unknown resource IDs, zero/negative intervals, and capacity exceedance fail.
pub fn resource_claims_fit(
    routes: &[&RouteOption],
    capacities: &[ResourceCapacity],
) -> bool {
    let mut boundaries = Vec::new();

    for route in routes {
        for claim in route.claims {
            if claim.start_tick >= claim.end_tick
                || resource_capacity(claim.resource_id, capacities).is_none()
            {
                return false;
            }
            boundaries.push(claim.start_tick);
            boundaries.push(claim.end_tick);
        }
    }

    boundaries.sort_unstable();
    boundaries.dedup();

    for window in boundaries.windows(2) {
        let start = window[0];
        let end = window[1];
        if start >= end {
            continue;
        }

        let mut active_by_resource: Vec<(&str, u16)> = Vec::new();
        for route in routes {
            for claim in route.claims {
                if claim.start_tick <= start && claim.end_tick >= end {
                    if let Some((_, count)) = active_by_resource
                        .iter_mut()
                        .find(|(id, _)| *id == claim.resource_id)
                    {
                        *count += 1;
                    } else {
                        active_by_resource.push((claim.resource_id, 1));
                    }
                }
            }
        }

        for (resource_id, count) in active_by_resource {
            let Some(capacity) = resource_capacity(resource_id, capacities) else {
                return false;
            };
            if count > u16::from(capacity) {
                return false;
            }
        }
    }

    true
}

/// Check one complete allocation under every declared weather branch.
pub fn joint_schedule_is_robust(
    route_a: &RouteOption,
    route_b: &RouteOption,
    capacities: &[ResourceCapacity],
) -> bool {
    if route_a.aircraft_id != "aircraft-a"
        || route_b.aircraft_id != "aircraft-b"
        || !route_is_robust(route_a)
        || !route_is_robust(route_b)
    {
        return false;
    }

    resource_claims_fit(&[route_a, route_b], capacities)
}

/// Return aircraft B's robust routes while holding aircraft A's allocation fixed.
pub fn robust_options_for_b(
    route_a: &RouteOption,
    capacities: &[ResourceCapacity],
) -> Vec<&'static str> {
    ROUTES_B
        .iter()
        .filter(|route_b| joint_schedule_is_robust(route_a, route_b, capacities))
        .map(|route_b| route_b.route_id)
        .collect()
}

/// Exhaustively enumerate every complete, robust assignment in the frozen domain.
///
/// This is a finite reference oracle for the tiny fixture, not a scheduler
/// intended to scale to a fleet or to model continuous aircraft dynamics.
pub fn enumerate_joint_schedules(
    capacities: &[ResourceCapacity],
) -> Vec<JointSchedule> {
    let mut schedules = Vec::new();

    for route_a in ROUTES_A {
        for route_b in ROUTES_B {
            if joint_schedule_is_robust(&route_a, &route_b, capacities) {
                schedules.push(JointSchedule {
                    aircraft_a_route: route_a.route_id,
                    aircraft_b_route: route_b.route_id,
                });
            }
        }
    }

    schedules.sort_by_key(|schedule| {
        (schedule.aircraft_a_route, schedule.aircraft_b_route)
    });
    schedules
}

/// Reject a policy that selects different actions for indistinguishable delivered histories.
///
/// Hidden world branches do not authorize policy branching. Two decisions with
/// the same observation class must choose the same route, irrespective of
/// which evaluator-only world branch produced the hidden state.
pub fn is_non_anticipative(decisions: &[BranchDecision]) -> bool {
    for (index, left) in decisions.iter().enumerate() {
        for right in decisions.iter().skip(index + 1) {
            if left.observation_class == right.observation_class
                && left.route_id != right.route_id
            {
                return false;
            }
        }
    }
    true
}

/// Count-based option externality for aircraft B in this frozen identical domain.
///
/// The returned delta is a fixture-local cardinality difference, not a physical
/// safety margin or a comparable measure across heterogeneous aircraft.
pub fn option_externality_for_b(
    baseline_route_a: &RouteOption,
    counterfactual_route_a: &RouteOption,
    capacities: &[ResourceCapacity],
) -> i32 {
    let baseline = robust_options_for_b(baseline_route_a, capacities).len() as i32;
    let counterfactual =
        robust_options_for_b(counterfactual_route_a, capacities).len() as i32;
    counterfactual - baseline
}
