use shipyard_dispatch_audit::EvaluatorOutage;
use shipyard_dispatch_experiment::evaluate_four_rules;
use shipyard_observed_dispatch::{
    Commitment, DispatchInput, Operation, Resource, VisibleOutage,
};

fn main() {
    let policy_input = DispatchInput {
        decision_tick: 0,
        horizon_end_tick: 20,
        operations: vec![
            Operation {
                id: "weld-long".to_owned(),
                order: 0,
                release_tick: 0,
                due_tick: 10,
                duration_ticks: 3,
                resource_kind: "welder".to_owned(),
                predecessor_ids: vec![],
            },
            Operation {
                id: "weld-urgent".to_owned(),
                order: 1,
                release_tick: 0,
                due_tick: 8,
                duration_ticks: 2,
                resource_kind: "welder".to_owned(),
                predecessor_ids: vec![],
            },
            Operation {
                id: "paint-after-weld".to_owned(),
                order: 2,
                release_tick: 0,
                due_tick: 14,
                duration_ticks: 2,
                resource_kind: "paint".to_owned(),
                predecessor_ids: vec!["weld-long".to_owned()],
            },
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

    // This evaluator-side list remains separate from policy_input. A future
    // evaluator-only event can be added here without leaking into the policy.
    let evaluator_outages = vec![EvaluatorOutage {
        event_id: "synthetic-known-maintenance".to_owned(),
        resource_id: "weld-bay-1".to_owned(),
        effective_from_tick: 2,
        effective_until_tick: 5,
    }];

    let results = evaluate_four_rules(&policy_input, &[], &evaluator_outages)
        .expect("policy and evaluator histories must agree");
    println!("synthetic_episode=yard-recovery-smoke-v0");
    println!("note=synthetic plumbing test only; not industrial evidence");
    println!("metric_directions=lower makespan/tardiness/occupied ticks are not collapsed into one score");
    for result in results {
        println!(
            "rule={:?} policy_status={:?} audit_status={:?} makespan_tick={:?} tardiness_ticks={} occupied_resource_ticks={} assigned={} unscheduled={} violations={}",
            result.rule,
            result.policy_status,
            result.audit.status,
            result.audit.metrics.makespan_tick,
            result.audit.metrics.total_tardiness_ticks,
            result.audit.metrics.occupied_resource_ticks,
            result.audit.metrics.assigned_operation_count,
            result.audit.metrics.missing_operation_ids.len(),
            result.audit.diagnostics.len()
        );
    }
}
