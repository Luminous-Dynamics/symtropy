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


## First-order component uncertainty (V0.1)

The crate also exposes a first-order uncertainty reference for the net electrical boundary:

`P_net = rho * g * Q_turbine * H_net * eta_turbine * eta_generator - P_aux`

All six input standard uncertainties are required: water density, turbine branch flow, net head, turbine efficiency, generator efficiency and auxiliary power. Missing any one returns `Ok(None)`; it is never treated as zero. Each uncertainty is individually bound to a source/evidence class, method, observation or assessment window, instrument or map identity, revision and calibration-validity statement. The report retains all six provenance records with the analytic Jacobian, signed per-input contributions, combined standard uncertainty and declared independence assumption.

Important limits:
- V0.1 assumes mutually independent standard uncertainties. It does not infer independence from sensor IDs or shared calibration systems.
- It does not produce an expanded uncertainty or confidence interval, and does not make a probabilistic service guarantee.
- The uncertainty report cannot override the water-service or evidence-eligibility gates; its `comparison_eligible` field reflects the existing hydraulic result gates.
- Net-head uncertainty must already account for the caller's gross-head / conveyance-loss method; covariance between those inputs belongs in the later covariance-aware extension.
- V0.2 will add explicit covariance with PSD checks and (u^2(P)=J\Sigma J^T). V0.3 will handle bounded/nonlinear maps and asymmetric distributions.

Additional tests check the analytic known-answer uncertainty, finite-difference Jacobian agreement, missing uncertainty, zero uncertainty, invalid uncertainty values, retained provenance and the rule that an uncertainty calculation cannot lift an ineligible hydraulic result.
