# Orbital Foundry V0 — synthetic reference benchmark

This standalone Rust tool explores a deliberately tiny finite design space for a modular orbital-foundry concept. It is an **event-driven arithmetic, constraint, and Pareto-search reference contract**, not a spacecraft simulator.

## What it models

Sixteen combinations of four binary design decisions are compared:

- standardized versus bespoke module interfaces;
- isolated versus shared power domains;
- a carried spare pack versus no spare pack;
- an explicitly declared end-of-life plan versus no plan.

The versioned fixture includes five synthetic disturbance scenarios: robot-joint failure, interface mismatch, power-domain failure, delayed resupply, and a compound common-cause event with repeated power faults. Each scenario lists ordered fault events with synthetic ticks. Faults map to explicit services, and a repeated fault queues behind any active recovery on that service. The event trace records repair start/end, queue delay, spare inventory before/after, and recovery duration.

The toy spare pack contains two generic repair/consumable units. This is an explicit synthetic assumption to demonstrate resource contention in a compound event; it is not a parts list or a spacecraft logistics estimate. Event timestamps and recovery values are discrete synthetic tick values. Simultaneous events are applied in fixture order, which is a deterministic tie-break policy rather than a claim about real response scheduling.

## Run

From the repository root:

    cargo test --manifest-path tools/orbital-foundry-bench/Cargo.toml
    cargo run --manifest-path tools/orbital-foundry-bench/Cargo.toml

To bind a report to a source commit:

    cargo run --manifest-path tools/orbital-foundry-bench/Cargo.toml -- --source-commit "$(git rev-parse HEAD)"

The JSON report includes the exact fixture SHA-256, optional source commit, all 16 candidates, separate objective values, hard-gate results, per-scenario event traces, remaining spare inventory, independent-oracle status, and non-dominated feasible design IDs.

## Metrics and claim ceiling

The report keeps launch mass in **synthetic mass units (SMU)**, assembly duration in **synthetic ticks**, interface-family count, additive service-loss ticks, and elapsed recovery horizon separate. Additive loss sums outage across affected services; elapsed recovery measures from the first fault until the last queued repair completes. These answer different questions. No arbitrary combined score is used.

The hard gate requires a declared end-of-life plan and a worst-case service-loss value no greater than the fixture threshold. These are synthetic policy rules; they do not establish orbital disposal feasibility or regulatory compliance.

## Independent checks

The production event transition and the exact small-domain oracle use separate implementations. Tests cover all 16 configurations, shared spare consumption during a compound event, malformed or non-monotonic event sequences, unknown design configuration, strict fixture/schema checks, weakened thresholds, tampered metrics, a SHA-256 known-answer vector, reproducibility, and Pareto trade-offs.

The focused workflow checks formatting, runs the tests, emits a deterministic receipt bound to the exact checked-out commit, independently inspects receipt fields, and uploads the receipt.

## Out of scope

This contract does **not** validate orbital dynamics or conjunction probability; structural strength or fatigue; contact, collision, reachability, or robotic assembly physics; thermal control, radiation or micrometeoroid protection; empirical supply-chain behavior, launch cost, mass, or service probabilities; ECLSS, life-support chemistry or crew safety; orbital disposal compliance, spacecraft certification, or flight qualification.

## Industry comparison

This crate does not claim to beat NASA GMAT, OpenMDAO, NASA CARA methods, or any industrial spacecraft-design stack. Those tools operate at different layers. A meaningful comparison must align one subproblem at a time, pin reference inputs and solver settings, document applicability and tolerances, and independently reproduce every headline result.

The next increment should connect this lifecycle policy model to the existing exact shipyard staging/scheduling verifier, rather than creating another yard geometry engine. Higher-fidelity orbital, structural, thermal, robotics, and life-support solvers should be integrated only through explicit model-applicability and verification gates.
