# Mine System Benchmark (MINE-SYSTEM-001)

A small, standard-library-only Rust reference evaluator for the first accounting and admission contracts of the underground mine-campaign benchmark.

## Scope

This is deliberately a small semantic kernel, not an end-to-end physical mine simulator. It currently checks:
- finite, non-negative ledger values and declared residual tolerances;
- material-balance closure for one aggregate output stream per shift;
- explicit saleable-product mass reconciles with accepted saleable tonnes and does not exceed aggregate accounted mass output;
- reported energy intensity matches the declared end-use energy ledger divided by accepted saleable tonnes;
- explicit qualification of evidence and hard constraints, with unknown/stale/conflicting/out-of-domain evidence failing closed;
- product specification disposition and hard-constraint violations;
- metric-only Pareto comparisons require the same comparison-basis identity and must be applied only after both candidates are eligible.

It does not simulate rock fracture/cutting, wear, haulage, processing, water flow, ventilation, thermal systems, mine economics, SMRs or geotechnical behavior. Cost, water and emissions metrics are caller-supplied comparison values at this stage; they are not independently recomputed from dedicated sub-ledgers yet.

## Run locally

From a Rust toolchain compatible with edition 2024:

```sh
cargo test --offline --manifest-path crates/tools/mine-system-benchmark/Cargo.toml
cargo run --offline --manifest-path crates/tools/mine-system-benchmark/Cargo.toml --bin mine-system-benchmark
```

The package has no third-party dependencies and declares its own nested Cargo workspace so that these commands don't build the main Bevy workspace.

## Claim ceiling

The built-in example is synthetic and separately records the product mass (42,000 kg) that corresponds to its 42.0 saleable tonnes per shift. An `ELIGIBLE_SYNTHETIC` result means only that the provided synthetic values satisfy this prototype's declared gates. It is not a mine safety decision, engineering authorization, physical model validation, reserve/resource statement, economic feasibility result, or SMR approval.

See [MINE_SYSTEM_BENCHMARK_V0_1.md](../../../docs/research/MINE_SYSTEM_BENCHMARK_V0_1.md) and [issue #1933](https://github.com/Luminous-Dynamics/symtropy/issues/1933) for the broader benchmark contract.

## Next portfolio work

The campaign extension for reuse-first comparisons is tracked in [BROWNFIELD-UNDERGROUND-001](https://github.com/Luminous-Dynamics/symtropy/issues/1951): compare existing-asset reuse, rehabilitation, partial reuse plus new excavation, new build, and no-build/remediation under equivalent service needs.

## Coupled fleet and ventilation research

The [VENT-FLEET-001 workstream](https://github.com/Luminous-Dynamics/symtropy/issues/1961) compares diesel and battery-electric fleet/ventilation baselines, VOD, and a coupled schedule. It treats airflow, contaminant clearance, thermal limits, emergency ventilation and critical evidence as hard gates. Published efficiency results are scenario-specific research anchors, not portable mine-wide savings. This reference evaluator does not actuate equipment or certify mine safety.

## Communication degradation and emergency messages

The MINE-SYSTEM-001 spec now includes [MS-014 Communications degradation](../../../docs/research/MINE_SYSTEM_BENCHMARK_V0_1.md), composed with [Symthaea MINE-COMMS-001 #8010](https://github.com/Luminous-Dynamics/symthaea/issues/8010) and [FLOOD-EGRESS-001 #1963](https://github.com/Luminous-Dynamics/symtropy/issues/1963). This is a synthetic systems benchmark; it makes no mine-radio or safety certification claim.

Symthaea's [snapshot fault campaign](https://github.com/Luminous-Dynamics/symthaea/pull/8011) and the proposed [deterministic multi-hop trace replay](https://github.com/Luminous-Dynamics/symthaea/issues/8066) should share fault/message manifest identities. Snapshot outcomes remain distinct from time-varying event replay; physical channel calibration remains gated by [MINE-TTE-CHAR-001 #8047](https://github.com/Luminous-Dynamics/symthaea/issues/8047).

## Rock-breakage research

The specialized [ROCK-BREAKAGE-TOURNAMENT-001](https://github.com/Luminous-Dynamics/symtropy/issues/1952) work compares mechanical, hybrid and non-mechanical approaches using explicit evidence tiers and full-cycle output boundaries.


Physical through-the-earth (TTE) characterization and held-out test traces are tracked in [Symthaea MINE-TTE-CHAR-001 #8047](https://github.com/Luminous-Dynamics/symthaea/issues/8047). These are required before synthetic TTE scenarios can support stronger physical claims.

## Human mobility and assisted egress

Compose with [SUBTERRANEAN-HUMAN-MOBILITY-001 #2037](https://github.com/Luminous-Dynamics/symtropy/issues/2037) and its [research contract PR #2038](https://github.com/Luminous-Dynamics/symtropy/pull/2038). Keep person/PPE/load/device geometry and route/hazard evidence explicit, distinguish self-egress from casualty extraction, and count no-safe-route when required evidence is unknown. Synthetic routing is not a rescue or mobility-device safety claim.
