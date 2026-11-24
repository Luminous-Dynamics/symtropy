# SAFRB v0.6 — multi-carrier service resilience portfolio

**Status:** standalone research evaluator; synthetic fixtures only  
**Research cut:** 24 November 2026  
**Language:** Rust 2024, dependency-free  
**Parent:** SAFRB v0.5 fuel-to-electricity service boundary

## Paradigm shift: protect services, not a preferred energy carrier

A port, hospital, logistics hub or municipal service may depend on several energy sources: grid import, shore power, diesel generation, solar PV, wind and battery storage. A diesel-only stock model can assess one threat to service, but it cannot compare the resilience contribution of these complementary sources.

v0.6 evaluates a supplied multi-carrier energy schedule at a named electrical service-zone boundary. It keeps source types distinct, preserves upstream evidence references, enforces battery power and energy constraints, updates state-of-charge across periods, allocates supply to demand by priority, and reports curtailment and exact bus energy-balance residuals.

The model does **not** infer that 1 kWh of a battery, grid import and diesel generator are operationally interchangeable. Each source must arrive at the same named bus boundary, during the same period, within its own evidence and availability envelope.

## Model

- Net electrical supply observations are represented as lower/upper Wh bounds at a service-zone bus.
- Source carriers remain typed: grid import, shore power, diesel generator, solar PV, wind turbine or other.
- Every supply record links to an upstream artifact, such as a meter receipt or v0.5 generator-day result.
- Reported, derived, forecast or unqualified scenario sources are excluded from firm dispatch availability.
- Batteries have bounded opening SOC, energy capacity, charge/discharge power and lower/upper round-trip conversion efficiency.
- One explicit charge/discharge/idle action is supplied per battery and period. The enum and uniqueness gate prevent same-period simultaneous charge and discharge.
- Charging consumes bus energy; stored energy is bounded by charge efficiency. Discharging consumes battery SOC and supplies bounded net bus energy through discharge efficiency.
- Service demand is allocated by ascending priority within the same zone and period.
- Unused energy is reported as curtailment; accepted zone/period balances must have exactly zero residual.

## Important distinctions

1. **Fuel inventory:** litres by product grade and batch, handled in v0.2.
2. **Generator conversion:** chemical energy to net generator output in Wh, handled in v0.5.
3. **Multi-carrier service portfolio:** generator, grid, shore power, renewables and battery at a bus boundary, handled here.
4. **Physical power network:** voltage, frequency, reactive power, network losses, protection, transient stability and islanding are not modelled.
5. **Optimized dispatch:** this crate evaluates a fixed schedule. It does not choose an optimal schedule.

## Evidence boundary

- Measured sources require a structurally SHA-256-shaped digest reference.
- Synthetic scenario inputs are accepted only under `urn:safrb:fixture:`.
- Battery state-of-charge and charge/discharge parameter evidence must independently qualify whenever battery state is reported or the battery is active; an idle battery with unqualified SOC is not silently surfaced as a numeric portfolio state.
- Signatures, raw evidence hashes, physical meter calibration and source truth are not verified by this crate.
- Evidence class and upstream artifact identifiers are preserved in output; they do not confer authority.

## Run

```sh
cargo fmt --manifest-path Cargo.toml -- --check
cargo test --manifest-path Cargo.toml --locked
cargo clippy --manifest-path Cargo.toml --all-targets --locked -- -D warnings
cargo run --manifest-path Cargo.toml --release --locked
```

## Matched solver tournament

The exact baseline versions, frozen-input contract, scenario protocol, objective hierarchy, metrics, hardware requirements and future qualification gates are preregistered in [SOLVER_TOURNAMENT_PROTOCOL.md](research/SOLVER_TOURNAMENT_PROTOCOL.md), with a tiny case manifest at [benchmark_manifest.toml](research/benchmark_manifest.toml).

## Industry comparison and research targets

Do not claim to replace PyPSA, PSS/E, DIgSILENT PowerFactory, ETAP, OpenDSS or established microgrid controllers. The appropriate research role is an evidence-qualified service-boundary benchmark that can feed independently validated optimization and power-system tools.

Compare:
- fixed schedule accounting against independently metered service output;
- battery state transition and efficiency against manufacturer/cell test maps;
- energy-service allocation against HiGHS LP/MILP and PyPSA dispatch on identical source, storage, demand and outage states;
- robust scheduling against PyPSA stochastic/CVaR methods;
- port energy scenarios against Sol-Atlas' existing port/energy twin;
- generator-origin electricity against v0.5 with matched batch and generator evidence.

### Proposed qualification targets (not achieved results)

- Zero accepted bus energy-balance residuals.
- Zero battery SOC, capacity or power-envelope violations in accepted schedules.
- 100% exclusion of forecast/reported-only supplies from firm energy.
- No simultaneous charge and discharge per battery/period.
- Zero demand overdelivery.
- On a calibrated independent test set, 90% empirical coverage for nominal 90% supply/service intervals.
- In a matched portfolio tournament, at least 10% lower CVaR95 of unserved critical-service energy than a deterministic baseline at equal/lower cost, with hard safety and quality constraints unchanged.

Targets are future gates. The present executable values are synthetic fixture data and cannot establish real-world performance.

## Claim ceiling

Passing Rust checks qualifies the software's arithmetic for synthetic schedule inputs. It does not qualify a physical port, grid, generator, battery system, national energy supply plan or emergency operating procedure.
