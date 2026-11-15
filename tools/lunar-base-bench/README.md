# Lunar seed-base utility benchmark (v0.1)

A standalone, headless Rust engineering slice for the first integrated lunar seed-base mission: preserve declared critical electrical service while scheduling a deferrable batch resource process, and close energy and mass accounting independently.

This is designed to compose existing Symtropy work rather than replace it:
- [#1618 CRASH-BOOT-00](https://github.com/Luminous-Dynamics/symtropy/issues/1618) — settlement and capability bootstrap;
- [#1803 BENCH-SPACE-008](https://github.com/Luminous-Dynamics/symtropy/issues/1803) — resource-to-product pathways, mass balances, coproduct boundaries, evidence gating;
- [#1840 SOLAR-THERMAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/1840) — hand-checkable energy-to-service modelling;
- [#1736 space-economy suite](https://github.com/Luminous-Dynamics/symtropy/pull/1736) — lifecycle/cislunar service alternatives.

## Run

```sh
cargo test --manifest-path tools/lunar-base-bench/Cargo.toml
cargo run --manifest-path tools/lunar-base-bench/Cargo.toml -- \
  tools/lunar-base-bench/fixtures/lunar_seed_utility_case_v0_1.json \
  --source-commit "$(git rev-parse HEAD)" > lunar_seed_utility_receipt.json
```

The CLI binds the receipt to SHA-256 of the exact JSON input bytes, the source-design-basis bytes, the model schema, and the supplied source commit. Replays contain no timestamp or randomness.

## Model boundary and assumptions

The checked-in case has eight synthetic one-hour intervals, a fixed-efficiency battery, critical energy demand, and an indivisible batch process. A batch consumes its declared feed mass only when the complete energy demand is served. A postponed batch consumes no feed and produces no water.

Two policies receive identical inputs and component limits:

- `greedy_immediate`: runs a batch whenever present-day power and battery limits permit;
- `forecast_reserve`: runs a batch only when it is fully powered **and** the post-batch stored energy remains above the backward-computed reserve for future critical demand, assuming later declared generation and charging/discharging limits.

Efficiencies are integer parts-per-million and energy values are integer Wh. The future reserve calculation is a deterministic horizon calculation, not an optimized or calibrated controller. Critical demand is served before productive batches; there is no actuation authority or flight interface.

The synthetic process declares a 10,000 g feed batch, 100,000 PPM contained-water fraction, 800,000 PPM recovery, and 5,000 Wh batch energy. These values are chosen for hand-checkable regressions; **they are not lunar regolith properties, resource estimates, or measured extraction yields**.

## Independent verification

The verifier recomputes, for every interval:
- source generation plus initial stored energy against critical/productive service, final SOC, curtailment, and charge/discharge losses;
- the battery state transition, conversion losses, charge/discharge limits, and capacity;
- exact feed transformation into dry tailings, recovered water, and unrecovered contained water;
- aggregate totals from the individual trace, rather than trusting candidate-reported residuals.

A one-interval unexpected solar-generation loss is an adverse control: the report must expose any unmet critical energy. Correct accounting is not the same as robustness.

## Research references

- NASA [Moon Base Systems](https://www.nasa.gov/moonbase-systems/) describes mobility, communications/PNT, habitation, power and logistics as interacting system capabilities.
- NASA [Lunar Surface Technology](https://www.nasa.gov/lunar-surface-technology/) identifies surface power, thermal technology, autonomous robotics, dust mitigation, excavation/construction, communications/PNT, ISRU and environmental constraints.
- NASA [ISRU overview](https://www.nasa.gov/overview-in-situ-resource-utilization/) distinguishes potential resource occurrence from deposit characterization, accessibility, extraction and useful products.

These references establish engineering boundaries, not the numerical assumptions of this synthetic case.

## Proposed gates (not achieved results)

- zero energy-balance residual in every interval and zero mass-balance residual for every processed batch;
- zero critical energy shortfall in the nominal forecast-reserve fixture;
- a greedy counterexample that reveals the cost of consuming stored energy for early production;
- explicit, non-hidden critical-load shortfall under a seeded unexpected generation loss;
- deterministic exact-input receipts and fail-closed malformed-input checks.

No lunar site has been selected. This crate does not model solar geometry, detailed thermal physics, regolith excavation mechanics, dust degradation, radiation, habitat/ECLSS closure, communications, flight hardware, crew risk or autonomous flight safety. It is a small power-dispatch and mass-accounting regression, not a lunar-base design approval.
