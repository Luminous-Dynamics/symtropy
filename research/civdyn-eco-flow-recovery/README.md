# CIVDYN Eco-Flow Recovery v0.1

A synthetic repeated-shock experiment that composes the shared resource ledger to distinguish three separate outcomes:

1. **Household service recovery** — are essential household water requirements met?
2. **Environmental-flow recovery** — is a declared minimum water flow delivered to a non-human ecological-flow obligation?
3. **Reserve recovery** — has the source stock been replenished enough to withstand another shock?

This extends the service-vs-reserve experiment in PR #3781. It does not model ecological health, aquifer dynamics, biodiversity, habitat response or real-world hydrology.

## Core paradigm

A system can restore household service while:
- leaving its source reserve depleted;
- failing to deliver environmental flows;
- becoming more vulnerable to the next disturbance.

A single "service recovered" flag is therefore not an adequate civilizational resilience certificate.

## Frozen synthetic system

The fixture contains four resource-ledger nodes:
- One basin with 200 m³ opening water stock and no population.
- Two household nodes, each with one synthetic household and 12.5 m³ essential demand per tick.
- One wetland-flow-obligation node with one **obligation unit** and a declared 10 m³/tick environmental-flow demand. Its population value is a ledger device for expressing a required flow; it is not a human population or an ecological population estimate.

Water is the only resource. All flows are in m³. The shared civdyn-resource-ledger is the sole authority for production, transfers, consumption, stock and conservation.

## Frozen disturbance sequence

| Phase | Ticks | Basin recharge |
|---|---:|---:|
| Pre-shock | 2 | 35 m³/tick |
| First shock | 4 | 0 |
| Recovery | 2 | 35 or 55 m³/tick |
| Repeated shock | 6 | 0 |

Per tick, household demand is 25 m³ and environmental-flow demand is 10 m³.

Two recovery capital paths:
- service_only: recharge equals current total demand (35 m³/tick); service returns but reserve is not rebuilt.
- capital_rebuilding: recharge is 55 m³/tick; 35 m³ meets current demand and the extra 20 m³ replenishes source stock.

Two explicit allocation priorities:
- household_priority: serve the two household demands first, then the environmental-flow obligation.
- ecological_flow_priority: deliver the environmental-flow obligation first, then serve households.

The portfolio is the full 2 × 2 crossing. No policy is declared universally optimal; the purpose is to expose the service/ecological-flow trade-off and its dependence on available reserve capital.

## Separate measurements

Each run reports:
- household unmet water (m³);
- environmental-flow unmet water (m³);
- minimum household coverage;
- minimum environmental-flow coverage;
- reserve after first shock;
- reserve after the service-recovery interval;
- reserve recovery ratio relative to pre-shock stock;
- repeat-shock unmet water, separated into household and environmental-flow obligations;
- service and environmental-flow recovery latency;
- maximum node/tick conservation residual.

No composite score is emitted. Household service, environmental flow, and reserve stock are not interchangeable units or normative weights.

## Synthetic negative control

Under the capital-rebuilding path, the second six-tick shock begins with a 100 m³ reserve and requires 210 m³ in total.

- Household priority produces 75 m³ unmet household demand and 35 m³ unmet environmental flow.
- Ecological-flow priority produces 80 m³ unmet household demand and 30 m³ unmet environmental flow.

The 5 m³ shift is a controlled illustration of a policy trade-off, not an empirical estimate. It demonstrates why environmental-flow obligations must be measured separately rather than hidden inside household coverage.

## Claim boundary

This experiment demonstrates ledger-backed accounting and a deterministic allocation trade-off under a declared synthetic scenario. It does **not** establish:
- actual ecosystem recovery or ecosystem health;
- groundwater/aquifer recharge, storage dynamics or safe yield;
- environmental-flow requirements for any real catchment;
- a real-world policy effect, calibrated model or regional forecast;
- normative legitimacy or a universal priority ordering between human and ecological claims.

A real case requires catchment-specific source data, ecological-flow studies, calibrated recharge/storage dynamics, uncertainty ranges and independent hydrological/ecological validation.

## Run

```sh
cargo fmt --manifest-path research/civdyn-eco-flow-recovery/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-eco-flow-recovery/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-eco-flow-recovery/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-eco-flow-recovery/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-eco-flow-recovery/Cargo.toml --bin eco-flow-recovery-sweep
```
