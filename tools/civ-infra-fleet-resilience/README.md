# CIV-INFRA Fleet Resilience Manifest and Tiny Schedule Oracle

A standalone Rust tool for validating the v0.1 synthetic fleet-resilience scenario manifest and exhaustively solving three deliberately tiny, static scheduling instances. It is isolated from the production workspace and has no network or device-control path.

From the repository root:

```sh
cargo fmt --manifest-path tools/civ-infra-fleet-resilience/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civ-infra-fleet-resilience/Cargo.toml
cargo run --locked --manifest-path tools/civ-infra-fleet-resilience/Cargo.toml -- docs/research/civ-infra-resilience/fixtures/fleet-resilience-v0.1.json
```

## What the executable does now

1. Parses a strict versioned JSON schema and rejects unknown fields.
2. Checks unique IDs and cross-references for policies, resources, the task DAG, disturbance cases, and evidence vocabulary.
3. Rejects missing resources/evidence, task dependency cycles, invalid task/resource bounds, zero capacities/demands, invalid frozen-case counts, and observations that precede event injection.
4. Computes SHA-256 over the exact fixture bytes.
5. Exhaustively searches the explicitly bounded tiny schedule cases, including start times, precedence, alternative resource assignments, half-open tick intervals, and resource capacities.
6. Compares computed minimum makespans with the frozen expected values and emits a deterministic, normalized report with the selected schedule and number of feasible schedules enumerated.

The finite search is bounded to at most 5 tasks, a 6-tick horizon, at most 2 alternatives per task, and at most 4 resource demands per alternative. This is a reference oracle for small instances, not a general-purpose production scheduler. It uses the declared task durations and resource requirements; it does not infer engineering or physical capability.

## Frozen finite reference outcomes

- `O-001-parallel-branches`: minimum makespan 5 ticks.
- `O-002-resource-alternative`: minimum makespan 4 ticks by assigning preparation to `oracle-robot-a`, allowing `oracle-robot-b` to prefabricate in parallel.
- `O-003-horizon-infeasible`: no complete schedule fits the 3-tick horizon.

These expected values are test assertions. They are not construction productivity measurements.

## Claim ceiling

The report's `validation_status` is `fixture_schema_valid_and_finite_schedule_oracle_checked`, and `finite_schedule_oracle_executed` is true. It deliberately also sets:

- `benchmark_executed: false`
- `policy_runs_performed: 0`
- each disturbance case to `declared_not_simulated`

Thus the oracle executes, but it does not run B0/B1/B2 planning policies or simulate injected failures and recovery. It does not validate hardware, field productivity, construction safety, or dispatch authority. The independent scheduling oracle is not yet an independent implementation or third-party audit.

CI must run on the exact head before Rust execution is considered verified; queued or missing checks are not passes.

## First recovery-policy microcase

A separate binary now executes one frozen synthetic robot-loss rollout and compares three lanes:

- **B0 — protective-stop baseline:** an equipment-level interlock stops the campaign when the active task's resource fails; the planner is not granted early knowledge of the cause.
- **B1 — local repair-and-retry:** retains the original assignment and restarts the interrupted task when the resource is declared available again.
- **B2 — observation-gated replan:** after the fault becomes observable, reassigns the interrupted task to an explicitly listed alternative resource if it is currently available.

Run it from the repository root:

```sh
cargo run --locked --manifest-path tools/civ-infra-fleet-resilience/Cargo.toml --bin policy-rollout -- docs/research/civ-infra-resilience/fixtures/policy-rollout-microcase-v0.1.json
```

The frozen reference outcomes are B0: safe stop with no accepted synthetic output; B1: four tasks including final inspection completed at tick 8; B2: the same synthetic acceptance at tick 6 after one reassignment at tick 2. The two-tick difference is specific to this hand-authored case, not a performance estimate. The report hashes the exact fixture bytes and fails closed if policy outcomes do not match the fixture.

This extends the *execution boundary* beyond the manifest-only tool, but only to one scenario and a small hand-coded discrete-tick model. It is not yet a rollout of all 13 declared disturbances, a general planner, physics, hardware validation, real inspection evidence, safe dispatch, or field-productivity measurement. The Rust compile and tests must pass on the exact commit before this executable is considered verified.
