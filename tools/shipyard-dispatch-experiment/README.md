# Shipyard dispatch experiment adapter

This crate joins the policy-facing observed-history dispatch input to an
independent schedule evaluator while keeping the two information boundaries
separate.

- Policy receives DispatchInput, including only outages delivered by the
  decision tick.
- Evaluator receives source-owned completed-operation intervals and evaluator
  outage truth separately.
- Each dispatch rule receives the identical policy input.
- Every rule's output, including partial plans and rejected schedules, is retained
  in content-addressed method-configuration, candidate-schedule, and independent-audit sidecars.
- A result-ledger v0 JSONL row points to all three exact sidecar SHA-256 digests
  and stores separate metrics for makespan, tardiness, and occupied resource ticks.
- Input state and disturbance digests hash the exact fixture bytes; per-rule
  method-configuration digests hash a sidecar that records the base config digest,
  rule identity, revision, and declared policy input boundary.
  No weighted "winner score" is emitted.
- An abstract audit pass is recorded as `incomplete`, not `feasible`, because
  geometry, causal replay, and receipt-integrity gates are not run by this adapter.
  The synthetic example declares an illustrative budget profile but does not
  enforce its time or compute-unit limits; the outer benchmark runner must do that.

## Run

    cargo test --locked --offline --manifest-path tools/shipyard-dispatch-experiment/Cargo.toml
    cargo run --locked --offline --manifest-path tools/shipyard-dispatch-experiment/Cargo.toml --example compare_synthetic_rules

The included workload and outages are synthetic. This is a plumbing and
counterexample harness, not a yard-calibrated benchmark or a result against
industrial state of the art.

The example prints JSONL records to stdout and writes content-addressed
method-configuration/candidate/audit sidecars to `SHIPYARD_EVIDENCE_DIR` (or a process-specific
temporary directory). The ledger references their exact SHA-256 values. An outer
runner should preserve the JSONL plus those sidecars and bind both to an exact
source revision and the immutable episode/tape digests.

## Interpretation

AbstractFeasible is only feasibility under the independent evaluator's
unary-resource, interval, release, duration and precedence model. Spatial
polygon overlap, transport, swept paths, crane/vehicle kinematics, lift charts,
and physical safety remain outside this model. An evaluator-only disturbance
can make a candidate fail the audit even when it was not in the policy's
visible information prefix; this measures the realized outcome, not clairvoyance.
