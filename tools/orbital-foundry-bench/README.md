# Orbital Foundry V0 — synthetic reference benchmark

This standalone Rust tool explores a deliberately tiny finite design space for a modular orbital-foundry concept. It is an **arithmetic, constraint, and Pareto-search reference contract**, not a spacecraft simulator.

## What it models

Sixteen combinations of four binary design decisions are compared:

- standardized versus bespoke module interfaces;
- isolated versus shared power domains;
- a carried spare pack versus no spare pack;
- an explicitly declared end-of-life plan versus no plan.

The fixed synthetic scenario family covers robot-joint failure, interface mismatch, power-domain failure, delayed resupply, and a compound common-cause failure. The model intentionally uses **synthetic mass units (SMU)** and **synthetic time ticks**; these are not kilograms, seconds, empirical probabilities, or flight predictions.

The report keeps launch mass, assembly duration, interface-family count, the worst-case service loss, and the unweighted sum of service-loss ticks across scenarios as separate objectives. No arbitrary combined score is used. The hard gate requires a declared end-of-life plan and worst-case loss no greater than the fixture's threshold. This gate is only a synthetic policy rule; it does not establish orbital disposal feasibility or regulatory compliance.

## Run

From the repository root:

    cargo test --manifest-path tools/orbital-foundry-bench/Cargo.toml
    cargo run --manifest-path tools/orbital-foundry-bench/Cargo.toml

To bind a report to a source commit:

    cargo run --manifest-path tools/orbital-foundry-bench/Cargo.toml -- --source-commit "$(git rev-parse HEAD)"

The JSON output includes the exact scenario fixture's SHA-256, the optional source commit, every evaluated candidate, individual scenario-loss values, gate outcomes, an independent-oracle result, and the non-dominated feasible design IDs.

## Verification boundary

The model evaluator and the tiny-domain oracle are separate code paths. Tests compare all 16 configurations, reject unknown design fields, reject fixture-key drift, check the SHA-256 known-answer vector, detect tampered metrics, test end-of-life gate failure, confirm deterministic output, and preserve Pareto trade-offs.

A passing local contract establishes only that this small synthetic arithmetic model behaves according to its encoded rules. It does **not** validate:
- orbital dynamics or conjunction probability;
- structural strength, fatigue, pressure integrity, thermal performance, radiation or micrometeoroid protection;
- real robotic contact, collision, reachability or assembly dynamics;
- real supply-chain statistics, lifecycle costs, launch mass or service probabilities;
- environmental-control/life-support system behavior or crew safety;
- end-of-life disposal compliance, spacecraft certification, or flight qualification.

## Industry comparison

This crate does not claim to beat NASA GMAT, OpenMDAO, NASA CARA methods, or any industrial spacecraft-design stack. Those tools operate at different layers. A meaningful later comparison must align one subproblem at a time, pin reference inputs and solver settings, document applicability and tolerances, and independently reproduce every headline result.

The next engineering increment is to consume qualified outputs from the existing scheduling/staging work, then integrate physical-discipline solvers only through explicit and validated adapters. This tool does not implement another geometric shipyard scheduler.
