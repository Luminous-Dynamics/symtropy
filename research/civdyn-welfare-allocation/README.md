# CivDyn Welfare Allocation v0.1

A dependency-free exact small-instance allocation benchmark for scarce civilizational resources.

## Scientific purpose

The benchmark separates:
- verified marginal returns;
- unverified persuasive assertions;
- minimum essential-service floors;
- total productivity;
- recipient coverage distribution.

It is a synthetic decision benchmark. It does not define a universal social welfare function, justice criterion or public policy.

## Policies

- Verified utilitarian: maximize verified return while respecting declared essential floors.
- Max-min coverage: maximize the lowest normalized recipient coverage.
- Leximin coverage: lexicographically maximize the sorted coverage vector.
- Equal coverage: minimize coverage variance.
- Persuasion susceptible: deliberately vulnerable control that ranks using unverified asserted returns, while score reporting still uses verified returns.
- Floor-blind utilitarian: deliberately unsafe negative control that ignores essential floors.

An exact enumerator returns the small-instance allocation and Pareto frontier over verified return and minimum coverage.

## Run

```sh
cargo fmt --manifest-path research/civdyn-welfare-allocation/Cargo.toml --all -- --check
cargo check --locked --manifest-path research/civdyn-welfare-allocation/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-welfare-allocation/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-welfare-allocation/Cargo.toml --bin welfare-allocation-sweep
```

## Claim ceiling

The source motivation is the ACL Findings 2026 Social Welfare Function Leaderboard, which evaluates productivity/inequality trade-offs and persuasion sensitivity. This crate does not reproduce that LLM benchmark or claim model performance. It establishes inspectable reference objectives, hard-floor behavior and a rhetoric-only robustness control.
