# Mine flexible-load scheduling benchmark

Standalone Rust exact-oracle benchmark for **MINE-FLEX-001 v0.1**.

It compares a fixed schedule with bounded exhaustive scheduling over a tiny authored synthetic mine process. It keeps safety-critical electricity and protected-water reserves as hard constraints, combines period-specific renewable supply / grid import limits / price / carbon intensity / water, and reports both the minimum-cost plan and cost-carbon Pareto frontier.

Run from the repository root:

```sh
cargo fmt --manifest-path tools/mine-flex-scheduling-bench/Cargo.toml -- --check
cargo test --manifest-path tools/mine-flex-scheduling-bench/Cargo.toml
cargo run --quiet --manifest-path tools/mine-flex-scheduling-bench/Cargo.toml -- \
  docs/tech/mine-flex-scheduling-corpus-v0.1.json \
  --source-commit "$(git rev-parse HEAD)"
```

The corpus is at `docs/tech/mine-flex-scheduling-corpus-v0.1.json`. Every output is a regression test with arbitrary integer units, not a site cost model or real emissions measurement. Read [the benchmark contract](../../docs/tech/MINE_FLEX_SCHEDULING_BENCHMARK_V0_1.md) for scope, assumptions, claim ceiling and next evidence tier.
