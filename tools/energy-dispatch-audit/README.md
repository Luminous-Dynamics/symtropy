# Symtropy Energy Dispatch Independent Audit v0.1

This small, standalone Rust crate provides an independent audit of the **finite integer-quantized electrical schedule** emitted by Sol-Atlas's energy_dispatch_assess CLI.

It is intentionally independent of the optimizer's transition/search implementation. It rechecks each returned alternative against the raw dispatch request: input identity, chronological interval alignment, source bounds, outage grid restrictions, served/unserved demand, battery state transitions and efficiency arithmetic, charge/discharge limits, terminal reserve, bus energy balance, and objective totals. It also verifies the run receipt's SHA-256 binding to the exact request bytes and the compact serialized assessment.

## Run

    cargo test --manifest-path tools/energy-dispatch-audit/Cargo.toml

    cargo run --manifest-path tools/energy-dispatch-audit/Cargo.toml -- \
      tools/energy-dispatch-audit/fixtures/pv-outage.request.json \
      path/to/energy-dispatch-run-report.json

The run report is the JSON output emitted by Sol-Atlas's energy_dispatch_assess CLI, including both receipt and assessment. The CLI prints a machine-readable audit report; exit code 0 means receipt integrity and schedule invariants pass, 2 means audit failure, and 3 is reserved for an unknown status.

## Frozen fixture and tests

fixtures/pv-outage.request.json is copied from the synthetic Sol-Atlas request fixture at sol-atlas-core/schemas/examples/energy-dispatch-pv-outage.synthetic.json, source blob SHA d6c93b4448c8355c2732049ea9b3d133ee782b41. It declares two 1-hour intervals: 2 kWh of PV supply followed by 2 kWh of essential load during a grid outage, served by an idealized lossless 2 kWh battery.

Tests build a small known-answer report locally to test the auditor and then apply negative controls for grid import during outage, forged state of charge, changed request bytes, stale report digest, and incomplete solver claims. The test-generated report is **not** an output produced by running the Sol-Atlas optimizer. The dedicated workflow compiles/runs this auditor but does not yet execute the cross-repository integration.

## What a pass means — and does not mean

A pass means the exact request/report bytes match the run receipt and each returned schedule passes the independently recomputed invariants for this finite model. The output always records objective_claim_status = not_independently_verified: this tool does not prove exhaustive search, globally optimal results, or Pareto-frontier completeness. A solver's search_is_exact / pareto_frontier_complete flags are preserved as **solver-reported claims**, not certified by this verifier.

The audit does **not** establish AC power-flow feasibility, voltage/frequency stability, protection coordination, inverter dynamics, thermal/process behavior, real-world evidence truth, model calibration, hardware safety, field performance, or permission to actuate equipment. An unsupported physical/model requirement must remain outside the claim ceiling.

Related: [benchmark protocol PR #2728](https://github.com/Luminous-Dynamics/symtropy/pull/2728), [implementation issue #2729](https://github.com/Luminous-Dynamics/symtropy/issues/2729), [Sol-Atlas adapter PR #314](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/314).
