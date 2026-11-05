# CivDyn Cross-Scale Resilience v0.1

A deterministic trajectory benchmark built on the existing `civdyn-resource-ledger`. It does not create a second physical accounting engine.

## System boundary

Seven physical inventory nodes form three explicit scales:
- one basin source/reservoir;
- two district stores;
- four household/service nodes.

Parent nodes have zero population and no household demand. They store only their own physical inventory; they do not duplicate child inventories. Water (m³) and energy (kWh) remain separate throughout the ledger and results.

The scenario runs ten ticks: two pre-shock, four shock and four post-shock recovery. Three disturbances are evaluated:
- water supply shock;
- energy supply shock;
- compound water + energy supply shock.

The reference values are entirely synthetic. They are software fixtures, not an estimate or forecast for Johannesburg, Gauteng, South Africa or another location.

## Intervention policies

- `no_transfer`: negative control; supply at the basin cannot serve households without inter-region transfers.
- `proportional_solidarity`: allocate current deficits proportionally across districts and households.
- `deficit_first`: prioritize households by accumulated normalized water/energy service deficit.
- `reserve_protected`: proportional transfers while preserving half of each sender's opening stock under the existing ledger policy.

These are deterministic reference controllers, not learned agents or globally optimal strategies.

## Metrics

No composite resilience score is emitted. Each trajectory reports:
- total unmet water (m³) and total unmet energy (kWh) separately;
- water and energy coverage-deficit areas, in household-step fractions;
- minimum household coverage for each resource;
- recovery time after the shock, requiring all four households to meet the declared 0.95 service floor for two consecutive ticks;
- final basin water and energy stocks separately;
- unit-specific maximum conservation residuals;
- gross edge-transfer throughput per resource (the same physical stock may count once per edge on a multi-edge route; it is not a unique-volume total).

## Reproduction

```sh
cargo fmt --manifest-path research/civdyn-cross-scale-resilience/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-cross-scale-resilience/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-cross-scale-resilience/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-cross-scale-resilience/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-cross-scale-resilience/Cargo.toml --bin cross-scale-resilience-sweep
```

## Claim ceiling

The current implementation validates deterministic transfer scheduling, three-scale stock-flow conformance, separate water/energy deficits and sustained recovery detection on synthetic scenarios. It does not yet test stochastic uncertainty, endogenous infrastructure decay, maintenance debt, ecological regeneration, institutional latency, historical hindcasts or real operational data.

The next gate is to add shock-severity ensembles and at least 30 independent replications where inference is claimed, calibrate against source-backed regional data, and test endogenous as well as exogenous disturbances. Recovery to service level must not be mistaken for recovery of ecological stocks or long-term sustainability.


## Repeated-shock resource recovery capital

The `recovery_capital` module distinguishes two different recovery claims:

- **Service recovery:** all households meet the 0.95 service floor for two consecutive ticks.
- **Reserve recovery:** the basin stock is replenished relative to its pre-shock level.

Two synthetic trajectories have identical first-shock service and identical two-tick service recovery. The service-only path receives 25 m³/tick during recovery, exactly matching aggregate demand, so reserve remains at 100 m³. The capital-rebuilding path receives 50 m³/tick, serves 25 m³ and retains the net 25 m³/tick, reaching 150 m³ reserve.

A second six-tick supply interruption exposes the difference: the service-only path incurs 50 m³ unmet water demand; the capital-rebuilding path incurs zero. The reference is intentionally water-only and synthetic. It does not imply ecological recovery, renewable sustainability or a Johannesburg forecast.

Run the additional executable:

```sh
cargo fmt --manifest-path research/civdyn-cross-scale-resilience/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-cross-scale-resilience/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path research/civdyn-cross-scale-resilience/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-cross-scale-resilience/Cargo.toml --bin recovery-capital-sweep
```
