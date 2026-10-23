# Shipyard dispatch experiment adapter

This crate joins the policy-facing observed-history dispatch input to an
independent schedule evaluator while keeping the two information boundaries
separate.

- Policy receives DispatchInput, including only outages delivered by the
  decision tick.
- Evaluator receives source-owned completed-operation intervals and evaluator
  outage truth separately.
- Each dispatch rule receives the identical policy input.
- Every rule's full output—including partial plans and rejected schedules—is
  retained together with independent diagnostics and separately recomputed
  makespan, tardiness and occupied ticks. No weighted "winner score" is emitted.

## Run

    cargo test --locked --offline --manifest-path tools/shipyard-dispatch-experiment/Cargo.toml
    cargo run --locked --offline --manifest-path tools/shipyard-dispatch-experiment/Cargo.toml --example compare_synthetic_rules

The included workload and outages are synthetic. This is a plumbing and
counterexample harness, not a yard-calibrated benchmark or a result against
industrial state of the art.

## Interpretation

AbstractFeasible is only feasibility under the independent evaluator's
unary-resource, interval, release, duration and precedence model. Spatial
polygon overlap, transport, swept paths, crane/vehicle kinematics, lift charts,
and physical safety remain outside this model. An evaluator-only disturbance
can make a candidate fail the audit even when it was not in the policy's
visible information prefix; this measures the realized outcome, not clairvoyance.
