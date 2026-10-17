# Solar–thermal research kernel (prototype)

This standalone Rust crate implements the first executable slice for
[SOLAR-THERMAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/1840)
and the [solar–thermal benchmark contract](https://github.com/Luminous-Dynamics/symtropy/pull/1839).

## Included

- Interval-integrated solar energy split into optical loss, receiver loss, and useful heat.
- A sensible store with a piecewise-linear specific-heat curve and trapezoidal enthalpy integration.
- Power-bounded charge/discharge, explicit-Euler standing loss, and a minimum heat-exchanger approach-temperature gate.
- Separate verifier functions that recompute reports from inputs and check energy closure.
- Hand-calculable cases and tampered-report negative controls.

## Model limitations

The cp(T) curve is a supplied model input, not a materials database. Standing loss uses the interval-start temperature and intentionally omits ambient heat gain, wall transients, and stratification. The heat exchanger uses a single effectiveness parameter and a minimum-approach gate, not a distributed exchanger model. Incident solar input is already interval-integrated energy; GHI/DNI conversion and collector geometry are outside this kernel. These assumptions must not be mistaken for field validation.

All absolute temperatures are kelvin; energy is interval-integrated joules; power is watts; duration is seconds; specific heat is J/(kg K). Store internal energy is defined relative to the configured minimum temperature.

## Run locally

\`\`\`sh
cargo test --manifest-path research/solar-thermal-kernel/Cargo.toml
cargo run --manifest-path research/solar-thermal-kernel/Cargo.toml --example solar_thermal_receipt
cargo clippy --manifest-path research/solar-thermal-kernel/Cargo.toml --all-targets
\`\`\`

The authoring environment has no Rust/Cargo toolchain, so no compile or test pass is claimed. The tests are authored and must run on the exact PR head before compiler-verified status is claimed.

## Not included

Weather adapters, heat-pump COP maps, stratified/distributed storage, phase-change or thermochemical materials, optimization, hardware control, calibration, or deployment interfaces.
