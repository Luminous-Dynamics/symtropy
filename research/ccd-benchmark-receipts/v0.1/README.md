# CCD Benchmark Receipts v0.1

A dependency-free Rust contract for validating collision-detection benchmark results before they enter a leaderboard or public performance claim.

## Validated dimensions

- Dataset identity, version, source URL and SHA-256 checksum string; algorithm identity and configuration SHA-256 bind the exact tested build and method settings.
- Algorithm name/version/source commit, a canonical configuration SHA-256 (covering method parameters, feature flags and runtime settings), and execution environment (OS, architecture, CPU, physical cores, worker threads, compiler, build profile and optional GPU/driver pair).
- Explicit motion and precision profiles.
- Execution outcome. Timeouts, OOM and aborts remain reportable but cannot qualify as completed benchmark results.
- Eligible-pair oracle denominator, forwarded true/false positives, certified-pruned pairs and exact disposition conservation.
- One accepted and replayed certificate for every certified prune.
- Rejected certificate attempts counted in invalid fallback; unsupported, invalid and overflow fallbacks reported separately.
- Zero false negatives against the independent oracle.
- p50/p95/p99 timing quantiles for total step, broadphase, swept-bound producer, certificate verifier, narrowphase and solver.

The validator rejects false negatives, missing prune evidence, incomplete denominators, missing fallback reasons and incomplete runs presented as completed results. Zero denominators return unavailable rates rather than a fabricated zero.

## Claim boundary

This crate validates in-memory receipt structures. It does not download or parse external datasets, execute algorithms, authenticate upstream files, independently create the geometric oracle, or generate timing data. SHA-256 and source-commit strings are declarative metadata that the benchmark runner must verify. This is not a cryptographic signature.

## External benchmark target

The primary external target is the NYU Continuous Collision Detection project:

- 2021 handcrafted degeneracies and simulator-generated CCD queries: https://doi.org/10.1145/3460775
- 2024 full scene/time-of-impact archive with exact ground truth packages: https://archive.nyu.edu/handle/2451/74508
- Scalable CCD project, with the broadphase/narrowphase tournament over linearized trajectories: https://continuous-collision-detection.github.io/scalable_ccd/

The public project reports evaluation of 13 algorithms on linearized trajectories and releases analytic ground truth, implementations and a test framework. The receipt schema is designed to report recall, false-positive rate, fallback, proof coverage and p50/p95/p99 costs on matched strata. A linearized-trajectory result is not automatically evidence for Symtropy's Q16.16 integrator or rotational motion.

A complementary 2026 SIGGRAPH line studies floating-point-robust CCD for parametric surfaces using time-dependent inclusion and rational ground-truth queries. Its project page states code and data are pending publication, so track it as a future comparator rather than a current executable baseline: https://xw-c.github.io/publication/sig26/

## Run

```sh
cargo fmt --manifest-path research/ccd-benchmark-receipts/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/ccd-benchmark-receipts/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/ccd-benchmark-receipts/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Promotion targets (not achieved results)

- 100% independent pair recall on the supported subset.
- 100% replayable certificate coverage for certified-pruned pairs.
- Zero accepted invalid, stale or corrupted certificates.
- Zero unsupported/invalid/overflow inputs converted to prunes.
- Median swept-envelope volume inflation <=2.0x against the reference envelope.
- Certificate verification below 5% of total culling work.
- At least 20% less narrowphase work on two held-out workload families, with no more than 5% p95 end-to-end regression and no false negatives.

All timings must be matched on declared hardware, compiler, settings and thread count. CPU and GPU runs are separate. Different motion and precision models must not be merged into one score.
