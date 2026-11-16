# Material Flow Benchmark contract tool

A standalone Rust benchmark toolkit combining strict scenario contracts, handoff-state validation, analytical convergence and a reduced-order Symtropy physics episode.

## What it checks

- Schema version and strict unknown-field rejection.
- Opaque identifier shape and SHA-256 / Git commit digest shape.
- Unique actor, asset, location, segment, action, event and fault identifiers.
- Package-to-tote and tote-to-asset axis-aligned fit, container gross-mass and asset payload-mass limits.
- Cargo handling/seal class against final storage class.
- Actor roles, receiver endpoint, grant validity window and required actions.
- Transfer-chain continuity and declared asset/mode compatibility.
- Mandatory safety, authorized-destination, package-identity, household-privacy and unauthorized-mutation gates.
- Required event declarations and fault-injection references.

## Distributed physical handoff reference state machine

`src/transaction.rs` supplies a deterministic reference state machine for:
- purpose/time/audience/endpoint/scope/use-count checks at the authorization boundary;
- one logical release command ID with duplicate suppression;
- independently authored source release, receiver receipt and custody acknowledgement;
- event ID replay/conflict detection and monotonic sequence checks;
- safe hold before any dispatch, versus reconciliation-required after an uncertain physical action;
- explicit reconciliation by a configured third-party custodian, with two distinct evidence digests and a terminal outcome (completed, no action taken, returned or quarantined).

Reconciliation closes the *record*, not the physical transaction atomically. It does not re-arm the old command: a new attempt requires a new transaction and a fresh grant. The state machine does not authenticate Holochain signatures, establish that two evidence sources are truly independent, or drive actuators.

## L1 stationary-receiver physics episode

The `mfb-simulate` binary runs a reduced-order package drop into an open-top tote using the existing `symtropy-physics` engine:
- 3D rigid-box proxy for the package;
- static rigid-box tote base and walls derived from internal/external dimensions;
- fixed 480 Hz timestep (1,920 steps over four seconds) with the physics crate's `deterministic-net` Q16.16 quantization feature;
- command tape and per-tick bitwise world snapshots;
- independent same-host replay comparison;
- rotated package world-space envelope, containment, settling, numerical-invariant and maximum-contact-penetration checks (current gate: 5 mm);
- custody state-machine completion only after the simulated placement and replay checks pass.

Run it from the repository root:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-simulate -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json
```

The result prints an L1 JSON record containing the final package position, speed, trace SHA-256, replay equality, containment/settling results and transaction state. The CLI exits non-zero if the episode is not accepted.

This is an actual headless rigid-body simulation, but it is a **reduced-order stationary receiving scene**, not a complete logistics simulation. It does not model package deformation, a robot/gripper, carrier dynamics, aircraft, routes, sensor noise, actuator control, human exposure, or physical metrology. Bitwise replay is asserted only for two runs on the same host/build; cross-platform determinism is not claimed.


### Historical L1 contact baseline and current qualification

The first 120 Hz run is retained as a historical baseline: workflow [43768458267](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43768458267), 53 tests, 10.256 mm peak contact penetration. It is not the current acceptance threshold or current qualified setting.

The same four-second nominal scene was refined to 240 Hz (5.023 mm) and then 480 Hz / 1,920 steps (2.307 mm). The current hard demonstrator gate is **5.000 mm**. The 480 Hz setting passed exact-head workflow [43769459756](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43769459756), source `2643fc9df9c22dfa760179c1deab0cf2e7f16a0e`; see the measured history in the benchmark specification and follow-on exact-head run below.

### Contact-depth refinement record

The first 120 Hz L1 baseline measured 10.26 mm maximum contact penetration. Refining the same four-second episode to 240 Hz reduced it to 5.023 mm; the 480 Hz / 1,920-tick episode then reduced it to **2.307 mm** and passed the hard 5 mm L1 gate. The 480 Hz exact-head result passed [workflow 43769459756](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43769459756), source `2643fc9df9c22dfa760179c1deab0cf2e7f16a0e`, with the same-host replay, placement and custody gates intact. Do not substitute the earlier 40 mm permissive demonstrator threshold for the current 5 mm gate.

### Executed ten-fault protocol campaign (2026-11-16)

Exact-head workflow [43819492450](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43819492450) passed on source `a270a138095f187b6ad034fbb530cc27f746ec4f` with **61 Rust tests**, strict formatting, Clippy `-D warnings`, nominal and campaign-manifest validation, all ten protocol fault executions, four-timestep analytical convergence, L1 receiving simulation/replay and evidence upload.

The executed campaign reported:
- **10/10 episodes accepted** against their expected terminal-state oracle;
- pre-dispatch faults never issued a release command;
- replay of the same logical command was suppressed;
- a second distinct physical-release observation forced reconciliation;
- missing acknowledgement, power interruption and network partition after release all blocked blind retry;
- no injected fault was falsely reported as completed.

Evidence artifact: `mfb-l1-evidence-a270a138095f187b6ad034fbb530cc27f746ec4f` (artifact ID `14204140876`). It contains `protocol-fault-campaign.json`, `free-fall-convergence.json`, `result.json` and environment metadata. The campaign is **protocol-level typed-event injection into an in-memory state machine**, not physical sensor/network/power fault injection, HIL or field testing.

## Tier-B free-fall convergence check

The `mfb-validate-convergence` binary evaluates Symtropy's translational integrator against the closed-form uniform-gravity solution over four matched one-second runs:

- (dt) = 1/30, 1/60, 1/120 and 1/240 seconds;
- position and velocity error for every run;
- relative mechanical-energy drift and rotation/numerical-health diagnostics;
- monotonic position-error reduction and an estimated log2 convergence order.

Run it from the repository root:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-validate-convergence
```

This is an analytical pre-contact translational check (Tier B for that narrow claim). It does not validate contact response, packaging, rotational inertia, robot handling or end-to-end delivery. Its result is archived beside the L1 receiving episode in the CI evidence artifact.

## Executable protocol fault campaign

The schema validator and the execution runner intentionally have different jobs:

- `mfb-validate-campaign`: checks that the frozen ten-fault matrix is structurally valid.
- `mfb-run-campaign`: executes each typed event sequence against a fresh handoff state machine and emits a machine-readable report.

The v1 runner checks:
- wrong container, over-mass and expired grant are rejected before dispatch;
- stale temperature, interlocked transfer gate and damaged/unknown seal enter safe hold before dispatch;
- replaying the same logical command is suppressed;
- a second physical release observation causes reconciliation rather than a second accepted release;
- missing receiver acknowledgement, power interruption and network loss after release require reconciliation;
- no ambiguous post-dispatch case permits a blind retry, and no injected fault is falsely recorded as completed.

Example result fields include `all_expected_outcomes_observed`, `pre_dispatch_faults_never_dispatched`, `post_dispatch_ambiguities_blocked_retry`, per-episode traces and observed transaction states.

## What it does not prove

The contract validator does not authenticate grant signatures, query Hearth or Commons, inspect a live asset, solve general 3D packing, prove safety, or qualify HIL / field performance. The separate L1 simulator executes one reduced-order stationary receiving scene; a valid manifest or successful L1 episode is not a physical safety case or field qualification.

All identifiers in public fixtures are synthetic opaque test identifiers. Do not place names, addresses, real DIDs, household inventories or raw sensor values in public scenarios.

## Commands

From the repository root:

```bash
cargo fmt --manifest-path tools/material-flow-benchmark/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --all-targets
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-validate-convergence
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-validate -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json
```

The fixture is a synthetic L1 manifest, not an executed simulation result.

Validate the ten-fault campaign against the nominal scenario:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-validate-campaign -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json \
  tools/material-flow-benchmark/fixtures/ten-fault-campaign.json
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-run-campaign -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json \
  tools/material-flow-benchmark/fixtures/ten-fault-campaign.json
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-simulate -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json
```

`mfb-validate-campaign` proves only that the ten episodes and expected terminal states form a consistent test plan. It does not execute faults. The separate `mfb-run-campaign` binary executes the ten typed fault events against a fresh instance of the Rust handoff state machine and verifies the expected terminal states and negative controls: pre-dispatch failures never dispatch, same-command replay is suppressed, duplicate physical release evidence enters reconciliation, and post-dispatch ambiguity blocks blind retry.

This is **protocol-level event injection**, not physical fault injection: it does not cut power, partition a live network, corrupt a sensor, move a real package or operate an actuator. Its report is included in the CI evidence artifact.
