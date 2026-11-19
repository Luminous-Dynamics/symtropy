# Marine Finite-Horizon Trajectory Viability Audit

Standalone Rust 1.96-compatible sampled-trajectory evaluator for Symtropy #6261.

## Commands

```sh
cargo fmt --manifest-path tools/marine-trajectory-viability/Cargo.toml -- --check
cargo clippy --manifest-path tools/marine-trajectory-viability/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/marine-trajectory-viability/Cargo.toml
cargo run --manifest-path tools/marine-trajectory-viability/Cargo.toml -- audit-trajectory tools/marine-trajectory-viability/fixtures/synthetic-trajectory-pass-0001.json
cargo run --manifest-path tools/marine-trajectory-viability/Cargo.toml -- audit-trajectory tools/marine-trajectory-viability/fixtures/synthetic-arrival-safe-midvoyage-breach-0001.json
```

## What the evaluator checks

- A digest-bound mission snapshot, eleven-axis service contract, strict shared time grid and exactly five disturbance classes.
- Conservative resource/service floors at every sampled time point, with lower bounds for minimum reserves and upper bounds for maximum exposures.
- Component availability and correlated common-cause outage identity.
- Energy, freshwater and waste-capacity conservation per interval, reconciled to the corresponding start/end resource states.
- No energy credited from an unavailable main generator; no water credited from an unavailable watermaker; no shore charge credited when shore power or the battery inverter is unavailable.
- Minimum conservative margin, time of minimum margin, first breach timestamp and maximum balance residual by resource.

The synthetic suite includes a passing five-scenario path and an adversarial path where arrival freshwater is above its floor but the reserve dips below that floor at the intermediate six-hour point. The latter must return `BLOCKED` and `INFEASIBLE/ABSTAIN`.

## Evidence boundary

This is a sampled finite-horizon audit, **not** a mathematical viability-kernel proof. It does not establish inter-sample safety, validate hydrodynamic or equipment models, estimate scenario probabilities, certify battery/watermaker performance, or authorize navigation. Physical deployment requires measured model validation, uncertainty calibration, additional disturbance coverage, independent review and HIL/PHIL testing.


## Industry and research positioning

Recent integrated ship-routing work already combines ensemble weather, adaptive hexagonal search, traffic-separation topology and edge-level intact-stability pruning. A 15 October 2026 paper reports that optimizing on a failed deterministic forecast can expose a ship to more adverse weather than weather-blind routing, and that member-coherent CVaR captures exposure hidden by marginal statistics (Zhang et al., DOI: https://doi.org/10.1016/j.oceaneng.2026.127661).

Recent all-electric ship work also couples voyage scheduling and power-system dispatch with a two-stage day-ahead/intraday optimization strategy (Gao et al., Journal of Energy Storage, 30 June 2026, DOI: https://doi.org/10.1016/j.est.2026.122206). Dual-layer MPC work for a diesel-LNG-battery vessel similarly targets the long/short-horizon energy-management tradeoff (Energy, 1 February 2026, DOI: https://doi.org/10.1016/j.energy.2025.139792).

These are important comparators, not strawmen. The additional research target here is a common finite-horizon verifier across multiple resource/service dimensions, with explicit common-cause disturbance branches and conservation ledgers, rather than treating power management or route completion as the whole mission.

## Benchmark ladder

1. **Trajectory integrity:** 100% exact time-grid coverage across every required scenario.
2. **Resource floors:** zero accepted conservative-bound breaches at any sampled time.
3. **Conservation:** 100% energy, freshwater and waste balances reconcile within preregistered units/tolerances.
4. **Equipment causality:** zero energy/water production credited while the supplying component is unavailable.
5. **Common cause:** zero independent-failure substitution for the required correlated outage branch.
6. **Evaluation:** whole-voyage/weather-event grouped forward-chaining holdouts, with p50/p95/p99 evaluator latency and per-resource worst margin.
7. **Physical progression:** independently reviewed model validation, HIL/PHIL, then sheltered-water trials before any operational safety claim.

Do not compare this sampled verifier to a formally proven viability kernel. That requires validated dynamics, admissible controls, a disturbance set, inter-sample guarantees and recursive-feasibility proofs. This first tool audits declared samples and ledgers only.
