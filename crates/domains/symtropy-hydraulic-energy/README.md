# Symtropy Hydraulic Energy Reference v0.1

A small, dependency-free Rust workspace package for analytically checking steady-state water-to-electric energy recovery under an explicit water-service contract.

## Scope

The model consumes:
- water density, upstream source flow, turbine-branch flow, bypass flow, gross head and declared conveyance head loss;
- an explicit water-flow allocation check: upstream flow must equal turbine flow plus bypass flow within a declared measurement tolerance;
- one evidence-bound turbine/PAT map cell with an explicit in-domain head/flow rectangle;
- an evidence-bound generator efficiency and auxiliary load;
- paired baseline/candidate water-service observations;
- an operator-approved contract declaring minimum service and maximum permitted regression.

It calculates:

`P_hydraulic = rho * g * Q * H_net`

then:
- turbine shaft output and turbine loss;
- generator gross output and conversion loss;
- auxiliary consumption and candidate net output;
- hydraulic/electrical balance residual;
- pressure, delivered-flow and ecological-flow service violations;
- a downstream allocation gate: delivered service flow plus reserved ecological flow cannot exceed measured upstream supply.

A candidate's calculated output is **not credited** when the protected water-service gate fails.

## Deliberate limits

This package is not:
- a hydrology model or resource atlas;
- a CFD / SPH / pipe-network solver;
- a water-hammer, surge, cavitation or transient safety solver;
- a turbine/PAT manufacturer curve database;
- an engineering design, licence, water-use approval or site suitability authority;
- a dispatch/portfolio optimiser;
- a pumped-storage model.

No map interpolation or extrapolation is performed: the exact operating point must lie inside the declared evidence-backed map cell. Transient hydraulic safety requires a specialist validated model and rig/field evidence. Pumped storage is a distinct storage topology and must not be counted as primary generation.

## Verify

From repository root:

```sh
cargo fmt --manifest-path crates/domains/symtropy-hydraulic-energy/Cargo.toml -- --check
cargo test --locked -p symtropy-hydraulic-energy
cargo clippy --locked -p symtropy-hydraulic-energy --all-targets -- -D warnings
```

The crate contains 20 analytical and adversarial tests, including exact steady-state power, flow-split closure, bypass-only service preservation with zero generation, downstream over-allocation rejection, head-loss inversion, turbine-map extrapolation, auxiliary-load bounds, and water-service / ecological-flow regression gates.

## Evidence and claim ceiling

An `EvidenceReference` records the source ID and declared evidence class; it does not authenticate a publisher or prove measurement quality. A service contract requires the caller to declare an `OperatorApprovedContract` evidence class, but this does not verify the underlying operator approval or legal authorization.

A passing analytic fixture proves only that the declared steady-state equations and checks are implemented consistently for that fixture. It does not establish project feasibility, field performance, pressure-surge safety, environmental compliance, grid connection or bankability.
