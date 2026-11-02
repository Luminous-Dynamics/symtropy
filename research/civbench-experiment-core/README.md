# CivBench Experiment Core

A small, dependency-free Rust qualification library for the Johannesburg CivBench experiment protocol.

This crate is deliberately **not** another simulator. It does not run EPANET, model hydraulics, parse the public JSON manifest, select policy, optimize sensors, or authorize water operations. It checks the contracts around a comparison so that later results cannot silently mix different exogenous shocks, initial states, topologies, model families, time grids or budgets.

## What it implements

### 1. Paired scenario contract

A control and treatment arm must share:
- scenario-block ID;
- exogenous event-schedule digest;
- initial-state digest;
- topology digest;
- structural model-family digest;
- time-grid digest;
- evaluation horizon;
- the same registered intervention factor, with distinct factor levels.

Resource envelopes must match across arms unless a changed resource dimension is explicitly declared as part of the registered treatment. This prevents an accidental budget mismatch from masquerading as a policy effect.

Evaluator-only truth is stored on the validated pair and excluded from the candidate-visible projection. Candidate policy code should receive only the candidate projection. The independent evaluator retains the truth digest separately.

### 2. Finite resource ledger

Resources have explicit integer units:
- water: litres;
- capital and operating budgets: declared minor currency units;
- crew and contractor capacity: minutes;
- parts and sensors: whole declared units;
- energy: watt-hours.

Every commitment has a unique identifier. The ledger checks every resource dimension before mutating state. Duplicate IDs, arithmetic overflow and over-budget requests leave the ledger unchanged. Create one ledger per policy arm and use the same declared resource envelope for paired comparisons unless the resource delta is the registered treatment.

### 3. Non-composite outcome receipt

The outcome vector retains separate fields for:
- essential water shortfall;
- service interruption exposure;
- worst-group service performance;
- physical losses;
- commercial losses;
- unbilled authorised consumption;
- asset failures;
- independently verified repairs;
- whole-life cost;
- recovery time;
- safe fallback delivery;
- false dispatches.

The paired receipt calculates signed treatment-minus-control deltas in every field. It does not select weights, collapse outcomes into one score or infer causality. Outcome definitions, denominators and threshold rules remain controlled by the experiment protocol.

### 4. Fail-closed provenance

A qualified receipt requires SHA-256 digests for:
- protocol and schema;
- source inventory and frozen source snapshots;
- model and topology;
- scenario generator and exact scenario;
- policy and independent evaluator;
- output artifact;
- runtime environment.

It also requires the exact code commit and solver/runtime version. Digests are fixed-size values; a missing or malformed digest cannot be represented. In particular, a source inventory whose source bytes remain **not_snapshotted** cannot be promoted into a qualified confirmatory receipt by inventing a placeholder digest.

## Run

~~~sh
cargo fmt --manifest-path research/civbench-experiment-core/Cargo.toml --all -- --check
cargo test --manifest-path research/civbench-experiment-core/Cargo.toml --locked
cargo clippy --manifest-path research/civbench-experiment-core/Cargo.toml --all-targets --locked -- -D warnings
~~~

The crate has no third-party dependencies. It is included in the standalone research-crates workflow.

## Limitations and claim ceiling

- This crate validates pair/ledger/receipt contracts; it does not execute scenario simulation or calculate hydraulic truth.
- The JSON protocol manifest and this Rust API are not yet bound by a generated schema adapter.
- Candidate/evaluator process isolation is a deployment responsibility; a Rust type boundary alone is not a security sandbox.
- Passing unit tests establishes only the declared software invariants.
- No field data, municipal operational authority, water-quality guarantee or causal service-impact claim is created by this crate.
