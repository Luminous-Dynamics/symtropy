# ORS kinetic-energy control-volume reference v0.1

**Evidence class:** analytical/software reference with synthetic frozen inputs  
**Tracking:** Symtropy [ORS-MECHANICAL-ENERGY-CONTROL-VOLUME-001 #5884](https://github.com/Luminous-Dynamics/symtropy/issues/5884)  
**Companions:** linear momentum #5872 and fixed-origin angular momentum #5877; parent #5387.

## Purpose

This standalone Rust reference checks the kinetic-energy balance for a finite open control volume. It computes kinetic energy carried by incoming/outgoing mass flow from each port's inertial velocity and keeps all power terms separate.

~~~text
K_CV = integral_CV(0.5 * rho * |v|^2 dV)

kinetic_energy_flux =
    sum_out(mdot * 0.5 * |v|^2)
    - sum_in(mdot * 0.5 * |v|^2)

dK_CV/dt + kinetic_energy_flux =
    nonport_external_power_on_CV
    + boundary_work_on_CV
    - mechanical_dissipation_rate
~~~

All terms are watts. Mass flow is a non-negative magnitude with an explicit inflow/outflow direction. The scenario carries frame, boundary and epoch identities. Ports are evaluated in that one inertial frame and boundary.

## Important interpretation

- Non-port external power is signed. A negative value is preserved as energy removed from the declared mechanical CV.
- Boundary work is a distinct signed power term; it is not combined with advective port kinetic-energy flux.
- Mechanical dissipation is a finite, non-negative sink in this kinetic-energy balance.
- A negative mechanical power is **not** evidence of electrical recovery, storage or efficiency.
- Port kinetic-energy flux uses absolute inertial velocity, while the energy balance is bounded by the declared mechanical control volume.

The reference does not close the first law or total system energy. Potential energy, elastic/strain energy, heat storage, electrical conversion/storage, field energy, radiation, and detailed thermodynamics are excluded. A physical application must derive these separately.

## Frozen corpus

Six cases and fourteen mass-flow ports cover:

1. Equal-speed straight-through flow.
2. Speed change with a hand-computable six-watt kinetic-energy flux.
3. Unsteady mass storage with simultaneous kinetic-energy accumulation.
4. Separate boundary work and mechanical dissipation.
5. Negative non-port external power without a recovery inference.
6. Three-dimensional multiple-inlet/multiple-outlet flow.

Tests also cover rotation invariance of kinetic-energy flux, canonical port-order summation under fractional values, deterministic receipt generation, mass/power mismatch and invalid dissipation/flow values.

The JSON receipt binds the exact source commit and both CSV SHA-256 digests. It records each port's specific kinetic energy and signed power flow as well as the scenario's separate balance terms.

## Run

~~~sh
cargo fmt --manifest-path tools/ors-kinetic-energy-control-volume-reference/Cargo.toml --all -- --check
cargo clippy --manifest-path tools/ors-kinetic-energy-control-volume-reference/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path tools/ors-kinetic-energy-control-volume-reference/Cargo.toml --locked --all-targets

SOURCE_COMMIT="$(git rev-parse HEAD)"
CASES_SHA="$(sha256sum tools/ors-kinetic-energy-control-volume-reference/cases-v0.1.csv | awk '{print $1}')"
PORTS_SHA="$(sha256sum tools/ors-kinetic-energy-control-volume-reference/ports-v0.1.csv | awk '{print $1}')"
cargo run --quiet --manifest-path tools/ors-kinetic-energy-control-volume-reference/Cargo.toml --locked -- \
  --source-commit "$SOURCE_COMMIT" --cases-sha256 "$CASES_SHA" --ports-sha256 "$PORTS_SHA"
~~~

## Claim ceiling

A pass establishes only that kinetic-energy accumulation, advective kinetic-energy flux, signed declared power and non-negative mechanical dissipation close for the frozen pressureless synthetic cases. It does not establish orbital-ring energy sufficiency, recovered electrical power, efficiency, thermal viability, field energy, mechanical survivability, feasibility or safety.
