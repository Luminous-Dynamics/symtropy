# Material Flow Benchmark contract tool

A standalone, dependency-light Rust contract parser and validator for MFB scenario manifests.

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
- fixed 120 Hz timestep with the physics crate's `deterministic-net` Q16.16 quantization feature;
- command tape and per-tick bitwise world snapshots;
- independent same-host replay comparison;
- final package envelope, containment and settling checks;
- custody state-machine completion only after the simulated placement and replay checks pass.

Run it from the repository root:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-simulate -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json
```

The result prints an L1 JSON record containing the final package position, speed, trace SHA-256, replay equality, containment/settling results and transaction state. The CLI exits non-zero if the episode is not accepted.

This is an actual headless rigid-body simulation, but it is a **reduced-order stationary receiving scene**, not a complete logistics simulation. It does not model package deformation, a robot/gripper, carrier dynamics, aircraft, routes, sensor noise, actuator control, human exposure, or physical metrology. Bitwise replay is asserted only for two runs on the same host/build; cross-platform determinism is not claimed.


### First L1 baseline (2026-11-15)

The exact-head workflow run [43768458267](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43768458267) passed the 53-test Rust suite, Clippy, both fixture validators and the nominal L1 simulation. The L1 evidence artifact is `mfb-l1-evidence-c2fa555623bdb8a242d238c3c2a0d6cf6885a0e1` (artifact ID `14183207787`), containing raw `result.json` and `environment.txt`.

Observed nominal outputs: package centre `[-0.0024414, 0.0455627, -0.0003204] m`; final linear/angular speed `0`; same-host bitwise replay `true`; safe containment `true`; transaction `completed`; accepted `true`; non-finite bodies `0`; maximum contact penetration `0.0102563 m` (10.26 mm); rotation orthogonality error `3.33e-15`. The 10.26 mm penetration is a baseline to improve, not a high-fidelity contact claim. The code currently permits 40 mm in this demonstrator; issue [#5469](https://github.com/Luminous-Dynamics/symtropy/issues/5469) tracks tighter contact/convergence validation.

## What it does not prove

The contract validator does not authenticate grant signatures, query Hearth or Commons, inspect a live asset, solve general 3D packing, prove safety, or qualify HIL / field performance. The separate L1 simulator executes one reduced-order stationary receiving scene; a valid manifest or successful L1 episode is not a physical safety case or field qualification.

All identifiers in public fixtures are synthetic opaque test identifiers. Do not place names, addresses, real DIDs, household inventories or raw sensor values in public scenarios.

## Commands

From the repository root:

```bash
cargo fmt --manifest-path tools/material-flow-benchmark/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --all-targets
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-validate -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json
```

The fixture is a synthetic L1 manifest, not an executed simulation result.

Validate the ten-fault campaign against the nominal scenario:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-validate-campaign -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json \
  tools/material-flow-benchmark/fixtures/ten-fault-campaign.json
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-simulate -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json
```

Campaign validity proves only that the ten test episodes and their expected terminal states form a consistent test plan. It does not prove that any simulated or physical fault has been injected or recovered.
