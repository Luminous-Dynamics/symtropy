use shipyard_observed_dispatch::{
    Commitment, DispatchInput, DispatchRule, Operation, Resource, Tick, VisibleOutage,
    build_dispatch_plan,
};

fn operation(
    id: &str,
    order: u64,
    due_tick: Tick,
    duration_ticks: Tick,
    resource_kind: &str,
    predecessor_ids: &[&str],
) -> Operation {
    Operation {
        id: id.to_owned(),
        order,
        release_tick: 0,
        due_tick,
        duration_ticks,
        resource_kind: resource_kind.to_owned(),
        predecessor_ids: predecessor_ids
            .iter()
            .map(|item| (*item).to_owned())
            .collect(),
    }
}

fn main() {
    let input = DispatchInput {
        decision_tick: 0,
        horizon_end_tick: 20,
        operations: vec![
            operation("weld-long", 0, 10, 3, "welder", &[]),
            operation("weld-urgent", 1, 8, 2, "welder", &[]),
            operation("paint-after-weld", 2, 14, 2, "paint", &["weld-long"]),
        ],
        resources: vec![
            Resource {
                id: "weld-bay-1".to_owned(),
                kind: "welder".to_owned(),
            },
            Resource {
                id: "paint-booth-1".to_owned(),
                kind: "paint".to_owned(),
            },
        ],
        completed: Vec::new(),
        commitments: Vec::<Commitment>::new(),
        visible_outages: vec![VisibleOutage {
            event_id: "synthetic-known-maintenance".to_owned(),
            resource_id: "weld-bay-1".to_owned(),
            effective_from_tick: 2,
            effective_until_tick: 5,
            observed_at_tick: 0,
        }],
    };

    for rule in [
        DispatchRule::EarliestDueDate,
        DispatchRule::CriticalRatio,
        DispatchRule::LeastSlack,
        DispatchRule::FixedOrder,
    ] {
        let plan = build_dispatch_plan(&input, rule);
        println!("rule={rule:?} status={:?}", plan.status);
        for assignment in &plan.assignments {
            println!(
                "  operation={} resource={} interval=[{}, {}) frozen={}",
                assignment.operation_id,
                assignment.resource_id,
                assignment.start_tick,
                assignment.end_tick,
                assignment.frozen
            );
        }
        for diagnostic in &plan.diagnostics {
            println!(
                "  diagnostic={} operation={:?} detail={}",
                diagnostic.code, diagnostic.operation_id, diagnostic.detail
            );
        }
    }
}
