# Symtropy Energy Dispatch Independent Audit v0.1

This small, standalone Rust crate provides an independent audit of the **finite integer-quantized electrical schedule** emitted by Sol-Atlas's energy_dispatch_assess CLI.

It is intentionally independent of the optimizer's transition/search implementation. It rechecks each returned alternative against the raw dispatch request: input identity, chronological interval alignment, source bounds, outage grid restrictions, served/unserved demand, battery state transitions and efficiency arithmetic, charge/discharge limits, terminal reserve, bus energy balance, and objective totals. It also verifies the run receipt's SHA-256 binding to the exact request bytes and the compact serialized assessment.

## Run

    cargo test --manifest-path tools/energy-dispatch-audit/Cargo.toml

    cargo run --manifest-path tools/energy-dispatch-audit/Cargo.toml --bin symtropy-energy-dispatch-audit -- \
      tools/energy-dispatch-audit/fixtures/pv-outage.request.json \
      path/to/energy-dispatch-run-report.json

The run report is the JSON output emitted by Sol-Atlas's energy_dispatch_assess CLI, including both receipt and assessment. The CLI prints a machine-readable audit report; exit code 0 means receipt integrity, schedule invariants, and applicable bounded-reference checks pass, 2 means audit failure, and 3 is reserved for an unknown status.

The additional Rust binaries `energy_candidate_schema_validate`, `energy_compilation_receipt_verify`, and `energy_audit_report_gate` support the cross-repository workflow. Schema validation uses pinned `jsonschema = 0.60.0` in Rust; request/config digests are checked by reserializing typed Rust structures instead of relying on cross-language JSON re-serialization. The end-to-end workflow covers grid-only, PV+grid, and PV+battery+grid candidate templates, with expected feasible-schedule counts pinned at 1, 1, and 14 for the frozen two-interval fixture.

## Frozen fixture and tests

fixtures/pv-outage.request.json is copied from the synthetic Sol-Atlas request fixture at sol-atlas-core/schemas/examples/energy-dispatch-pv-outage.synthetic.json, source blob SHA d6c93b4448c8355c2732049ea9b3d133ee782b41. It declares two 1-hour intervals: 2 kWh of PV supply followed by 2 kWh of essential load during a grid outage, served by an idealized lossless 2 kWh battery.

Tests build a synthetic known-answer report locally to exercise the auditor, then apply negative controls for grid import during outage, forged state of charge, changed request bytes, stale report digest, and incomplete solver claims. The test-generated report is **not** an output produced by running the Sol-Atlas optimizer. A separate exhaustive enumerator independently recovers the 14 feasible complete schedules and three non-dominated objective vectors for this tiny case. The reference enumerator also has a deterministic 128-case generated suite checking repeatability and non-dominated frontier invariants, plus fail-closed budget/depth controls. Those property checks test the reference implementation; they are not a substitute for differential tests against actual Sol-Atlas reports. Reference enumeration is limited to at most 64 intervals and 100,000 separately counted expansions; exceeding either limit is UNKNOWN, never PASS. The standalone workflow compiles/runs this auditor's tests. The separate Energy Studio End-to-End Bridge workflow is designed to consume actual Symthaea CLI output for all three reference architectures (grid-only, PV+grid, PV+battery+grid), validate each candidate against the pinned Symtropy schema, compile each through Sol-Atlas, run the exact dispatch CLI, and audit each actual report. Expected feasible complete-schedule counts are frozen at 1, 1 and 14 for these synthetic cases. Its cross-repository run is not yet verified passing.

## What a pass means — and does not mean

A pass means the exact request/report bytes match the run receipt, every returned schedule passes independently recomputed invariants, and the bounded reference enumerator agrees on feasible-schedule count and Pareto objective vectors. When reference enumeration completes, objective_claim_status = verified_for_bounded_model_case applies only to that finite request. When its separate expansion cap is exceeded, the aggregate status is unknown rather than a pass. Solver search_is_exact / pareto_frontier_complete flags are preserved as **solver-reported claims**; they are not the basis of the independent reference result.

The audit's exact reference enumeration is intended only for small finite integer cases. It does **not** establish AC power-flow feasibility, voltage/frequency stability, protection coordination, inverter dynamics, thermal/process behavior, real-world evidence truth, model calibration, hardware safety, field performance, or permission to actuate equipment. An unsupported physical/model requirement must remain outside the claim ceiling.

Related: [benchmark protocol PR #2728](https://github.com/Luminous-Dynamics/symtropy/pull/2728), [implementation issue #2729](https://github.com/Luminous-Dynamics/symtropy/issues/2729), [Sol-Atlas adapter PR #314](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/314).
