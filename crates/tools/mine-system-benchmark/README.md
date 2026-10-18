# Mine System Benchmark (MINE-SYSTEM-001)

A small, standard-library-only Rust reference evaluator for the first accounting and admission contracts of the underground mine-campaign benchmark.

## Scope

This is deliberately a small semantic kernel, not an end-to-end physical mine simulator. It currently checks:
- finite, non-negative ledger values and declared residual tolerances;
- material-balance closure for one aggregate output stream per shift;
- accepted product tonnage does not exceed accounted mass output;
- reported energy intensity matches shift energy output divided by accepted saleable tonnes;
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

The built-in example is synthetic. An `ELIGIBLE_SYNTHETIC` result means only that the provided synthetic values satisfy this prototype's declared gates. It is not a mine safety decision, engineering authorization, physical model validation, reserve/resource statement, economic feasibility result, or SMR approval.

See [MINE_SYSTEM_BENCHMARK_V0_1.md](../../../docs/research/MINE_SYSTEM_BENCHMARK_V0_1.md) and [issue #1933](https://github.com/Luminous-Dynamics/symtropy/issues/1933) for the broader benchmark contract.
