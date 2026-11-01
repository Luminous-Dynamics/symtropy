# Earth-to-orbit adaptive capital benchmark v0.1

Standalone, dependency-light Rust reference evaluator for the staged-investment policy contract in #3270.

## Run

```sh
cargo test --locked --manifest-path tools/earth-orbit-capital-bench/Cargo.toml
cargo run --locked --manifest-path tools/earth-orbit-capital-bench/Cargo.toml
```

The crate has no third-party dependencies and declares its own Cargo workspace so it can be checked independently of the Bevy application workspace.

## Semantics

All CSV capacities and demands are synthetic service units per period. All cost fields are synthetic cost units; no currency conversion, discounting, tax, financing or price-year claim is implied. The first executable slice is intentionally bounded:

- Three decision epochs and finite capacity increments.
- Initial action is chosen before demand signals are observed.
- Follow-up actions are keyed only by the observed low/high signal; two trajectories share each signal to expose uncertainty after observation.
- Build capacity arrives after a declared lead time; leased capacity is available immediately.
- Evidence gates are non-compensable: unknown investment evidence yields `NeedsEvidence`; failed evidence makes investment policies infeasible. No-build remains eligible.
- Output is a robust objective vector (minimum delivered service, maximum unmet demand, maximum lifecycle cost), not an opaque weighted score.
- A separately written exhaustive oracle in tests recomputes outcomes and verifies the Pareto-policy set.
- All 30 rows are synthetic and carry no claim about actual corridor, customer, launch-market, or project performance.

The model does **not** implement real option valuation, probabilistic demand, financing, launch mechanics, rail capacity, or environmental assessment. It is a small decision-policy contract fixture, not an investment recommendation or general optimizer.

## Case families

The 30 frozen rows include no-build, operation-first, lease-and-observe, modular expansion, low uptake, high uptake, learning/market changes, bottleneck shifts, polar-orbit site controls, closures, tunnelling thresholds, evidence gaps, public/distribution gates, value-of-information, perfect-information gap, and probability-free robust decisions. Each row tests only the dimensions represented by this bounded v0.1 model; family names are coverage intent, not proof that all real-world phenomena are simulated.
