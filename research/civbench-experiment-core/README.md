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

The ledger separates **available**, **reserved**, **consumed**, and **remaining** quantities. Planned work reserves finite resources; completed work settles the reservation using actual consumption; cancelled work releases the reservation. Actual consumption may be lower than reserved, returning the unused balance, but cannot exceed the reservation. Every transition is atomic and IDs remain recorded after settlement/release to prevent reuse.

Each commitment has a unique, whitespace-normalized identifier and a per-commitment reservation/settlement record. Duplicate IDs, invalid lifecycle transitions, arithmetic overflow, over-budget requests and settlement overruns leave the ledger unchanged. This distinguishes planned capacity from realized activity and supports realistic repair-queue/fallback-resource accounting.

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

Every metric carries both a value and an evidence status: observed, derived, modelled, not measured, unsupported, withheld or not applicable. The first three require a numeric value; the latter four require no value. An observed zero is different from an unsupported or missing metric. Worst-group service is checked against its 0..=1,000,000 ppm domain.

The paired receipt calculates signed treatment-minus-control deltas only when both arms contain valid numeric values **with exactly the same evidence status**. A measured observation cannot be subtracted from a modelled value by default. Missing control, missing treatment, both missing and evidence-status mismatch are represented by distinct delta statuses; their numeric delta is None, never zero. Cross-status comparison requires a separately preregistered and versioned bridge. It does not select weights, collapse outcomes into one score or infer causality. Outcome definitions, denominators and threshold rules remain controlled by the experiment protocol.

### 4. Fail-closed provenance

A qualified receipt requires SHA-256 digests for:
- protocol and schema;
- source inventory and frozen source snapshots;
- model and topology;
- scenario generator and exact scenario;
- policy and independent evaluator;
- output artifact;
- runtime environment.

It also requires the exact 40- or 64-character hexadecimal code commit and solver/runtime version. Digests are fixed-size values; missing values cannot be represented, malformed hexadecimal digests are rejected, and all-zero unset sentinels are rejected. The receipt constructor also requires an explicit **Frozen** source-snapshot status. A source inventory still marked **not_snapshotted** cannot qualify for confirmatory evaluation. The caller must only assert Frozen after actually capturing and hashing the required source bytes; the crate does not independently fetch or authenticate those bytes.

## Run

~~~sh
cargo fmt --manifest-path research/civbench-experiment-core/Cargo.toml --all -- --check
cargo test --manifest-path research/civbench-experiment-core/Cargo.toml --locked
cargo clippy --manifest-path research/civbench-experiment-core/Cargo.toml --all-targets --locked -- -D warnings
cargo doc --manifest-path research/civbench-experiment-core/Cargo.toml --no-deps --locked
~~~

The crate has no third-party dependencies. Its exact-path workflow is [.github/workflows/civbench-experiment-core.yml](../../.github/workflows/civbench-experiment-core.yml), which runs formatting, locked check/test, strict Clippy and documentation builds for this standalone crate.

## Limitations and claim ceiling

- This crate validates pair/ledger/receipt contracts; it does not execute scenario simulation or calculate hydraulic truth.
- The JSON protocol manifest and this Rust API are not yet bound by a generated schema adapter.
- Candidate/evaluator process isolation is a deployment responsibility; a Rust type boundary alone is not a security sandbox.
- Passing unit tests establishes only the declared software invariants.
- No field data, municipal operational authority, water-quality guarantee or causal service-impact claim is created by this crate.


### 5. Decision-to-service evidence closure

The event-chain validator checks one intervention from first observation through:
1. source observation;
2. state estimate;
3. recommendation;
4. human authorization, rejection or deferral;
5. work order/action;
6. independent verification;
7. measured service outcome;
8. post-event model discrepancy review.

A rejected/deferred decision is a valid terminal state but cannot be followed by a work order or a claimed service outcome. An authorized chain is only classified as a verified service outcome when all eight ordered, linked stages are present. Event IDs must be unique, evidence digests nonzero, timestamps monotonic and each event linked to its immediate predecessor.

This v0.1 API handles one event per stage in a linear chain. It does not yet model branched work orders, partial repairs, repeated observations or multi-crew work graphs. Such workflows require a versioned event-graph extension rather than weakening this validator.

The chain validator does not verify the factual quality of observation data, whether the human had legal authority, whether a repair physically succeeded, or whether the service outcome was caused by the action. It verifies that the declared evidence sequence is complete and internally linked. Those other questions require independent evidence and a governed evaluation.
