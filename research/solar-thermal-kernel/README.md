# Solar–thermal research kernel (prototype)

This standalone Rust crate implements the first executable slice for
[SOLAR-THERMAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/1840)
and the [solar–thermal benchmark contract](https://github.com/Luminous-Dynamics/symtropy/pull/1839).

## Included

- Interval-integrated solar energy split into optical loss, receiver loss, and useful heat.
- A sensible store with a piecewise-linear specific-heat curve and trapezoidal enthalpy integration.
- Sensible thermal exergy relative to a declared reference temperature, integrated from the same cp(T) curve; specific exergy (J/kg) and total exergy (J) have distinct types.
- Power-bounded charge/discharge, explicit-Euler standing loss, and a minimum heat-exchanger approach-temperature gate.
- Separate verifier functions that recompute reports from inputs and check energy closure.
- A standalone, dependency-free receipt auditor that consumes the emitted JSON and independently recomputes the solar, charge, standing-loss, discharge, temperature, and sensible-exergy ledgers without calling the model verifier.
- Hand-calculable cases and tampered-report negative controls for both the kernel and the artifact auditor.

## Model limitations

The cp(T) curve is a supplied model input, not a materials database. Standing loss uses the interval-start temperature and intentionally omits ambient heat gain, wall transients, and stratification. The heat exchanger uses a single effectiveness parameter and a minimum-approach gate, not a distributed exchanger model. Incident solar input is already interval-integrated energy; GHI/DNI conversion and collector geometry are outside this kernel. These assumptions must not be mistaken for field validation.

All absolute temperatures are kelvin; energy is interval-integrated joules; power is watts; duration is seconds; specific heat is J/(kg K). Store internal energy is defined relative to the configured minimum temperature.

## Run locally

```sh
cargo fmt --manifest-path research/solar-thermal-kernel/Cargo.toml
cargo test --manifest-path research/solar-thermal-kernel/Cargo.toml --all-targets
cargo run --manifest-path research/solar-thermal-kernel/Cargo.toml --example solar_thermal_receipt
cargo run --manifest-path research/solar-thermal-kernel/Cargo.toml --example solar_thermal_receipt | cargo run --manifest-path research/solar-thermal-kernel/Cargo.toml --example solar_thermal_audit
cargo clippy --manifest-path research/solar-thermal-kernel/Cargo.toml --all-targets -- -D warnings
```

The demo report includes its assumptions—including a separate ambient temperature for standing-loss recomputation and a reference temperature for exergy—but explicitly declares `source_binding: not-source-hash-bound`; it is not a provenance-secure experimental receipt. The auditor is intentionally limited to the versioned flat scalar report; it is not a general JSON parser or a calibrated plant validator. The exact-head targeted workflow is `.github/workflows/solar-thermal-kernel.yml`. The authoring environment has no Rust/Cargo toolchain, so no local compile or test pass is claimed; check the exact-head workflow rather than relying on the root workspace CI, which intentionally does not include this nested research workspace. The tests are authored and must run on the exact PR head before compiler-verified status is claimed.

## Not included

Weather adapters, heat-pump COP maps, stratified/distributed storage, phase-change or thermochemical materials, optimization, hardware control, calibration, or deployment interfaces.
