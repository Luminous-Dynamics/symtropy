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

Robustness tournament (synthetic generated validation + held-out seeds):

```sh
cargo run --quiet --manifest-path tools/mine-flex-scheduling-bench/Cargo.toml -- \
  --robustness --source-commit "$(git rev-parse HEAD)"
```

This command compares the frozen baseline, a forecast-only nominal scheduler and an envelope-feasible minimax scheduler over 32 validation and 32 non-overlapping held-out seeds, with five paired disturbance worlds per seed. Its receipt reports feasibility, grid cost and carbon, oracle regret, 95th-percentile regret and a worst-5%-mean CVaR-style metric. Held-out seeds are reproducible generated scenarios—not blinded external validation. Read docs/tech/MINE_FLEX_ROBUSTNESS_BENCHMARK_V0_1.md for the full contract.

Causal adaptive recourse and out-of-envelope challenge run:

```sh
cargo run --quiet --manifest-path tools/mine-flex-scheduling-bench/Cargo.toml -- \
  --recourse --source-commit "$(git rev-parse HEAD)"
```

This reuses the same dispatch/accounting code and published uncertainty set, commits one period at a time as observations arrive, and evaluates three additional out-of-envelope stress worlds per seed. It compares separate cost-first and carbon-first lexicographic recourse objectives, checks that identical observed prefixes yield identical actions within each policy, reports incomplete production explicitly, and emits separate cost/regret/feasibility results by validation/held-out split and envelope class. See docs/tech/MINE_FLEX_RECOURSE_BENCHMARK_V0_1.md for the causal policy contract and claim ceiling.

Risk-budgeted robust Pareto frontier (static schedule chosen from the published uncertainty envelope only):

```sh
cargo run --quiet --manifest-path tools/mine-flex-scheduling-bench/Cargo.toml -- \
  --risk-budget --source-commit "$(git rev-parse HEAD)"
```

This sweeps explicit carbon headroom constraints (0%, 2.5%, 5%, 10%, 25%, and 100% above the minimum worst-case carbon attainable by any robust-feasible schedule), minimizes worst-case cost under each cap, and emits the nondominated robust cost/carbon frontier plus paired validation/held-out outcomes. The exact contract and claim ceiling are in [MINE-FLEX-PARETO-001](../../docs/tech/MINE_FLEX_RISK_BUDGET_BENCHMARK_V0_1.md). Synthetic integer units only; this is not a site-calibrated controller.
