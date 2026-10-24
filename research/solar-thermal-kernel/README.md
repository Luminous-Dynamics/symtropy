# Solar–thermal research kernel (prototype)

This standalone Rust crate implements the first executable slice for
[SOLAR-THERMAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/1840)
and the [solar–thermal benchmark contract](https://github.com/Luminous-Dynamics/symtropy/pull/1839).

## Included

- A typed direct-beam input adapter converts declared DNI, aperture area, incidence cosine, unshaded fraction, and interval duration into aperture-plane power and interval energy; an independent verifier rejects tampered values.
- An explicit interval-irradiation conversion maps direct-normal irradiation in Wh/m² to interval-average DNI in W/m², preserving interval energy and independently verifying the conversion. This is a unit conversion, not reconstruction of within-interval irradiance variation; geometry applied to interval averages remains a declared approximation.
- A scenario-weighted service-risk evaluator computes expected and worst unmet useful heat, normalized service-shortfall mean, weighted P95 and CVaR95, and probability of violating a declared service floor. It rejects invalid scenario probability mass, duplicate IDs and over-delivery, and has an independent recomputation path for tail metrics. A fixed hand-calculable scenario-risk fixture is now serialized in the canonical receipt and audited independently by the stdin auditor, including tampered-outcome and tampered-metric negative controls. This summarizes supplied scenario outcomes; it does not generate scenarios or optimize dispatch.
- Interval-integrated solar energy split into optical loss, receiver loss, and useful heat.
- A sensible store with a piecewise-linear specific-heat curve and trapezoidal enthalpy integration.
- An alternative analytic standing-loss integrator that solves the constant-ambient lumped-store loss equation across piecewise-linear cp(T) segments; its independent verifier checks elapsed time, floor events, state bounds, and energy closure. The existing explicit-Euler primitive remains unchanged for backwards-comparable fixtures.
- Sensible thermal exergy relative to a declared reference temperature, integrated from the same cp(T) curve; specific exergy (J/kg) and total exergy (J) have distinct types.
- Power-bounded charge/discharge, explicit-Euler standing loss, and a minimum heat-exchanger approach-temperature gate.
- A two-store sensible-heat cascade primitive that respects source temperature grade, exchanger approach, source discharge power, receiving-store capacity, and heat-exchanger effectiveness.
- Static sensible-heat pinch targeting for non-isothermal streams with constant integrated heat capacity: shifted-temperature intervals, minimum hot/cold utility targets, maximum process-recovery target, pinch-side temperatures, and an independent target verifier.
- Multi-period fixed-grade storage dispatch: explicit period ordering, loss-before-action state transitions, explicit idle/charge/discharge actions, service unmet heat, and an independent per-period plus whole-schedule first-law verifier.
- Separate recomputation verifiers for collector, charging, standing loss, discharge, cascade transfers, static pinch targets, and multi-period storage dispatch.
- A standalone, dependency-free receipt auditor that consumes the emitted JSON and independently recomputes the interval-BNI-to-mean-DNI conversion, direct-beam DNI-to-aperture-energy calculation, collector, charge, standing-loss, discharge, temperature, sensible-exergy, two-store cascade, static pinch-target, and two-period day/night ledgers without calling the model verifiers.
- Hand-calculable cases and tampered-report negative controls for both the kernel and the artifact auditor.

## Model limitations

The pinch solver is static and currently supports sensible non-isothermal streams with constant integrated heat capacity; it does not synthesize a heat-exchanger network or model phase-change streams. The temporal layer composes existing store mechanics across declared periods but is not an optimizer: it accepts one explicit action (idle, charge, or discharge) per period, applies standing loss before that action, and does not represent simultaneous charge/discharge, direct process recovery, heat pumps, forecasting, or uncertainty. It does not yet couple the time-expanded heat-grade network to pinch targets. The cp(T) curve is a supplied model input, not a materials database. The legacy temporal dispatch layer still uses explicit-Euler standing loss from interval-start temperature and omits ambient heat gain, wall transients, and stratification. The new analytic integrator is a separate alternative for constant ambient/conductance and lumped equilibrium; it also omits ambient heat gain and does not model wall transients or stratification. The heat exchanger uses a single effectiveness parameter and a minimum-approach gate, not a distributed exchanger model. The collector balance accepts interval-integrated incident energy. The direct-beam adapter can derive that quantity from declared DNI and geometry factors, but does not infer DNI from GHI, calculate sun position/tracking, resolve shadows itself, include diffuse or ground-reflected radiation, or model receiver/optical losses. These assumptions must not be mistaken for field validation.

All absolute temperatures are kelvin; energy is interval-integrated joules; power is watts; duration is seconds; specific heat is J/(kg K). Store internal energy is defined relative to the configured minimum temperature.

## Run locally

```sh
cargo fmt --manifest-path research/solar-thermal-kernel/Cargo.toml
cargo test --manifest-path research/solar-thermal-kernel/Cargo.toml --all-targets
cargo run --manifest-path research/solar-thermal-kernel/Cargo.toml --example solar_thermal_receipt
cargo run --manifest-path research/solar-thermal-kernel/Cargo.toml --example solar_thermal_receipt | cargo run --manifest-path research/solar-thermal-kernel/Cargo.toml --example solar_thermal_audit
cargo clippy --manifest-path research/solar-thermal-kernel/Cargo.toml --all-targets -- -D warnings
```

The demo report includes its assumptions—including an independent synthetic interval-BNI fixture (100 Wh/m² over 900 s -> 400 W/m² mean DNI) and direct-beam fixture (800 W/m² DNI, 10 m² aperture, 0.5 incidence cosine, 0.8 unshaded fraction, 3600 s -> 3.2 kW and 11.52 MJ), a separate ambient temperature for standing-loss recomputation, a reference temperature for exergy, an analytic two-store cascade fixture, a static pinch-target fixture, and the two-period day/night storage fixture—but explicitly declares `source_binding: not-source-hash-bound`; it is not a provenance-secure experimental receipt. The auditor is intentionally limited to the versioned flat scalar report; it is not a general JSON parser or a calibrated plant validator. The exact-head targeted workflow is `.github/workflows/solar-thermal-kernel.yml`. The authoring environment has no Rust/Cargo toolchain, so no local compile or test pass is claimed; check the exact-head workflow rather than relying on the root workspace CI, which intentionally does not include this nested research workspace. The tests are authored and must run on the exact PR head before compiler-verified status is claimed.

## Not included

Provenance-bound weather-file traces, solar-position/shadow geometry, diffuse/albedo irradiance, heat-pump COP maps, stratified/distributed storage, phase-change or thermochemical materials, time-expanded pinch/dispatch optimization, uncertainty-aware forecasts, hardware control, calibration, and deployment interfaces. The day/night schedule is included in the emitted flat JSON receipt; the separate auditor independently recomputes the fixed period transitions, losses, temperature gate, delivered/unmet service and whole-schedule first-law balance. This remains a synthetic two-period fixture, not a benchmark run on plant data. Explicit idle periods are exercised in Rust tests but not yet serialized in the canonical receipt.
