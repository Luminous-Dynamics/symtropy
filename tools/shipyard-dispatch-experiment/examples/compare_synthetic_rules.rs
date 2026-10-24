use shipyard_dispatch_audit::EvaluatorOutage;
use shipyard_dispatch_experiment::{build_result_ledger_bundle, evaluate_four_rules, ExperimentLedgerContext};
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
    let context = ExperimentLedgerContext {
        benchmark_release_id: "synthetic-dispatch-v0".to_owned(),
        episode_id: "yard-recovery-smoke-v0".to_owned(),
        replicate_id: "seed-0001".to_owned(),
        initial_state_sha256: "a".repeat(64),
        disturbance_tape_sha256: "b".repeat(64),
        method_config_sha256: "c".repeat(64),
        budget_profile_id: "no-search-deterministic-v0".to_owned(),
        wall_time_limit_ms: 1000,
        decision_compute_budget_units: 100,
        budget_contract: "synthetic illustrative profile; runtime limits are not enforced by this adapter".to_owned(),
        policy_revision: "synthetic-policy-bundle-revision".to_owned(),
        evaluator_revision: "synthetic-evaluator-bundle-revision".to_owned(),
        synthetic_example: true,
    };
    let bundle = build_result_ledger_bundle(&context, &results)
        .expect("synthetic context satisfies the ledger contract");
    let evidence_dir = std::env::var_os("SHIPYARD_EVIDENCE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("shipyard-dispatch-evidence-{}", std::process::id()))
        });
    std::fs::create_dir_all(&evidence_dir).expect("create evidence directory");
    for artifact in &bundle.artifacts {
        std::fs::write(evidence_dir.join(&artifact.file_name), &artifact.contents)
            .expect("persist content-addressed artifact");
    }
    eprintln!("note=synthetic plumbing test only; not industrial evidence");
    eprintln!("evidence_dir={}", evidence_dir.display());
    print!("{}", bundle.ledger_jsonl);
}
