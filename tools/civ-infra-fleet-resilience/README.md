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

## Typed event timeline audit

A separate binary audits the existing 13-case disturbance manifest as a deterministic event queue, with evaluator-only injections kept separate from a policy-visible observation projection:

```sh
cargo run --locked --manifest-path tools/civ-infra-fleet-resilience/Cargo.toml --bin event-timeline-audit -- docs/research/civ-infra-resilience/fixtures/fleet-resilience-v0.1.json
```

For v0.1, the timeline has 13 distinct typed disturbance kinds, 3 delayed observations, 4 total ticks of observation delay, a maximum delay of 2 ticks, and snapshots for ticks 0 through 7. In the changed-site-geometry case (R-008), the evaluator sees injection at tick 2 while the policy projection must not see the event until tick 4. Same-tick event and observation lists are sorted deterministically. The report hashes exact input bytes. Policy-visible entries contain the event ID/type, event name, observation tick, affected resource/task references, invalidation references, and required recovery evidence; they deliberately omit the evaluator's injection timestamp and hidden-state fields. The evaluator-only projection separately tracks injected-but-not-yet-observable events. The unit suite also runs **1,000 deterministic generated event-queue property cases** (13 synthetic events per case) to check visibility boundaries and invariance to input-order permutation. This is not 1,000 policy rollouts and makes no recovery-performance claim.

This binary audits event identity, references, timing, deterministic release ordering and projection separation. It deliberately reports `policy_simulation_executed: false` and `policy_runs_performed: 0`. It does not simulate recovery, task execution or the B0/B1/B2 policies. That remains the goal of [CIV-INFRA-ROLLOUT-002 #1870](https://github.com/Luminous-Dynamics/symtropy/issues/1870).

### Independent report verifier

The companion `event-timeline-check` binary independently reconstructs the expected timeline from the exact fixture bytes and checks the audit report before it may be treated as a valid timeline artifact:

```sh
cargo run --locked --manifest-path tools/civ-infra-fleet-resilience/Cargo.toml --bin event-timeline-audit -- docs/research/civ-infra-resilience/fixtures/fleet-resilience-v0.1.json > /tmp/event-timeline-report.json
cargo run --locked --manifest-path tools/civ-infra-fleet-resilience/Cargo.toml --bin event-timeline-check -- docs/research/civ-infra-resilience/fixtures/fleet-resilience-v0.1.json /tmp/event-timeline-report.json
```

It recalculates the byte digest, event case records, delay counts, evaluator-side hidden/injected sets and policy-visible observation frames, and rejects unknown report fields or a false claim that the timeline tool simulated policies. CI now runs it against the first binary's actual JSON output. This is a separately implemented checker in the same project, not an independent third-party audit. Its eight unit tests inject early visibility, a forbidden injection-time field, an input-digest mismatch, false policy-simulation claims, changed frozen case identity, input-array permutation, and unknown report fields.

The policy observation surface is implemented as a cursor in a private Rust module: its caller can only `advance_one_tick()` to receive the next frame. There is no API to request tick N or inspect the queued future observations; the frame's schema omits event injection time and hidden evaluator state. The full timeline in the audit report is an artifact for review, not a policy input. A separate `event-timeline-check` executable reconstructs and verifies that artifact.


## Typed recovery-admission contract

Run the typed synthetic contract suite from the repository root:

```sh
cargo run --locked --manifest-path tools/civ-infra-fleet-resilience/Cargo.toml --bin recovery-admission-contract
```

This contract introduces explicit task states and allowed actions, then checks whether an action is admissible against the policy-visible observation prefix, task-predecessor receipts, resource/capacity gates, current capability and calibration, localization, communication, energy reserve, utility-envelope status, authorization, inspection outcome, and current recovery/acceptance evidence. It distinguishes an immediate protective stop from a replanning decision: a stop can be admitted before the planner can observe the cause, while resumption and acceptance cannot use future observations.

The suite derives observation records from the same 13-case frozen fixture using `observable_tick`; injection timestamps are not exposed in the policy-facing projection. It includes positive and negative cases for every disturbance, plus controls for stale capability, unknown utility, declined approval, overcommitment, missing predecessors/evidence, future-dated evidence, failed inspection, terminal-task restart, duplicate observation references, and invalid reassignment. Failed inspection remains rejected until explicit rework and later successful reinspection evidence supports acceptance.

The report is explicitly `synthetic_only` and sets `policy_simulation_executed: false`, `policy_runs_performed: 0`, `physical_control_executed: false`, and `dispatch_authorized: false`. This tests an admission contract; it is **not** a B0/B1/B2 rollout, a live dispatch path, a field-safety result, or a productivity benchmark. Rust formatting, compile, tests and the report assertions must pass on the exact commit before this slice is qualified.


**Trust boundary:** the gate snapshot booleans and predecessor receipts in this slice are synthetic caller-supplied assertions. The contract reports `input_receipts_cryptographically_verified: false` and `immutable_history_store_executed: false`; it does not verify signatures, real sensor provenance, or durable append-only history. A later integration must validate content-addressed/version-bound evidence through an independent receipt verifier before using these types to authorize any real operation.
