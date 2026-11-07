//! Independent reference checks for the shared-resource accounting kernels.
//!
//! These tests intentionally use a different, direct implementation: iterate
//! each integer tick for concurrent capacity and sum each accounting window for
//! cumulative quotas. They do not call the production event-sweep/quota helpers
//! when computing expected results.

use maritime_service_chain_contract::{
    CapacityWindow, Resource, ResourceKind, ResourceUnit, ResourceUse, TickWindow,
};
use maritime_service_chain_fleet::{
    OperationPlacement, check_concurrent_capacity, check_cumulative_quota,
};

#[derive(Clone, Copy)]
struct UseSpec {
    start: u64,
    duration: u64,
    amount: u64,
}

fn resource(kind: ResourceKind, unit: ResourceUnit, windows: &[(u64, u64, u64)]) -> Resource {
    Resource {
        resource_id: "shared-resource".into(),
        kind,
        capacity_unit: unit,
        availability: windows
            .iter()
            .map(|(start, end, capacity)| CapacityWindow {
                window: TickWindow {
                    start_tick: *start,
                    end_tick: *end,
                },
                capacity: *capacity,
            })
            .collect(),
        evidence_ids: vec!["fixture-capacity".into()],
    }
}

fn operation(id: &str, spec: UseSpec, unit: ResourceUnit) -> OperationPlacement {
    OperationPlacement {
        demand_id: id.into(),
        participant_id: format!("participant-{id}"),
        alternative_id: "alt-1".into(),
        segment_id: "segment-1".into(),
        start_tick: spec.start,
        end_tick: spec.start + spec.duration,
        resource_uses: vec![ResourceUse {
            resource_id: "shared-resource".into(),
            amount: spec.amount,
            unit,
        }],
    }
}

fn operation_with_two_uses(
    id: &str,
    start: u64,
    duration: u64,
    first_amount: u64,
    second_amount: u64,
    unit: ResourceUnit,
) -> OperationPlacement {
    OperationPlacement {
        demand_id: id.into(),
        participant_id: format!("participant-{id}"),
        alternative_id: "alt-1".into(),
        segment_id: "segment-1".into(),
        start_tick: start,
        end_tick: start + duration,
        resource_uses: vec![
            ResourceUse {
                resource_id: "shared-resource".into(),
                amount: first_amount,
                unit,
            },
            ResourceUse {
                resource_id: "shared-resource".into(),
                amount: second_amount,
                unit,
            },
        ],
    }
}

fn reference_window_for_interval(resource: &Resource, start: u64, end: u64) -> Option<usize> {
    resource.availability.iter().position(|window| {
        start >= window.window.start_tick && end <= window.window.end_tick && start < end
    })
}

/// Independent tick-sweep: check full-window coverage, then count every active
/// use at each integer tick. No endpoint-event sorting is used.
fn reference_concurrent(resource: &Resource, operations: &[OperationPlacement]) -> bool {
    for operation in operations {
        for usage in &operation.resource_uses {
            if usage.resource_id != resource.resource_id {
                continue;
            }
            if usage.unit != resource.capacity_unit
                || reference_window_for_interval(resource, operation.start_tick, operation.end_tick)
                    .is_none()
            {
                return false;
            }
        }
    }

    for window in &resource.availability {
        for tick in window.window.start_tick..window.window.end_tick {
            let active: u128 = operations
                .iter()
                .filter(|operation| operation.start_tick <= tick && tick < operation.end_tick)
                .flat_map(|operation| operation.resource_uses.iter())
                .filter(|usage| usage.resource_id == resource.resource_id)
                .map(|usage| u128::from(usage.amount))
                .sum();
            if active > u128::from(window.capacity) {
                return false;
            }
        }
    }
    true
}

/// Independent window-sum implementation. Every use must fit one whole window;
/// each declared use is charged once to that window.
fn reference_cumulative(resource: &Resource, operations: &[OperationPlacement]) -> bool {
    let mut used = vec![0_u128; resource.availability.len()];
    for operation in operations {
        for usage in &operation.resource_uses {
            if usage.resource_id != resource.resource_id {
                continue;
            }
            if usage.unit != resource.capacity_unit {
                return false;
            }
            let Some(index) =
                reference_window_for_interval(resource, operation.start_tick, operation.end_tick)
            else {
                return false;
            };
            used[index] += u128::from(usage.amount);
        }
    }
    used.iter()
        .zip(&resource.availability)
        .all(|(amount, window)| *amount <= u128::from(window.capacity))
}

#[test]
fn concurrent_event_sweep_matches_independent_tick_sweep_for_all_tiny_pairs() {
    let resource = resource(
        ResourceKind::Lock,
        ResourceUnit::Lockages,
        &[(0, 3, 1), (3, 6, 2)],
    );
    let mut cases = Vec::new();
    for start in 0..6 {
        for duration in 1..=2 {
            for amount in 1..=4 {
                cases.push(UseSpec {
                    start,
                    duration,
                    amount,
                });
            }
        }
    }

    for left in cases.iter().copied() {
        for right in cases.iter().copied() {
            let operations = vec![
                operation("left", left, ResourceUnit::Lockages),
                operation("right", right, ResourceUnit::Lockages),
            ];
            assert_eq!(
                check_concurrent_capacity(&resource, &operations),
                reference_concurrent(&resource, &operations),
                "concurrent mismatch: left=({}, {}, {}), right=({}, {}, {})",
                left.start,
                left.duration,
                left.amount,
                right.start,
                right.duration,
                right.amount
            );
        }
    }
}

#[test]
fn cumulative_window_sums_match_independent_window_accounting_for_all_tiny_pairs() {
    let resource = resource(
        ResourceKind::WaterQuota,
        ResourceUnit::Millilitres,
        &[(0, 3, 5), (3, 6, 4)],
    );
    let mut cases = Vec::new();
    for start in 0..6 {
        for duration in 1..=2 {
            for amount in 1..=6 {
                cases.push(UseSpec {
                    start,
                    duration,
                    amount,
                });
            }
        }
    }

    for left in cases.iter().copied() {
        for right in cases.iter().copied() {
            let operations = vec![
                operation("left", left, ResourceUnit::Millilitres),
                operation("right", right, ResourceUnit::Millilitres),
            ];
            assert_eq!(
                check_cumulative_quota(&resource, &operations),
                reference_cumulative(&resource, &operations),
                "quota mismatch: left=({}, {}, {}), right=({}, {}, {})",
                left.start,
                left.duration,
                left.amount,
                right.start,
                right.duration,
                right.amount
            );
        }
    }
}

#[test]
fn event_sweep_matches_reference_when_one_operation_repeats_the_same_resource() {
    let resource = resource(ResourceKind::Lock, ResourceUnit::Lockages, &[(0, 5, 2)]);
    let operations = vec![
        operation_with_two_uses("a", 1, 3, 1, 1, ResourceUnit::Lockages),
        operation(
            "b",
            UseSpec {
                start: 2,
                duration: 2,
                amount: 1,
            },
            ResourceUnit::Lockages,
        ),
    ];
    assert_eq!(
        check_concurrent_capacity(&resource, &operations),
        reference_concurrent(&resource, &operations)
    );
    assert!(!check_concurrent_capacity(&resource, &operations));
}
