# ErgoStream microcosm implementation status

**Version:** 0.1 · **Date:** 2026-11-23

This directory contains a deliberately small, dependency-free Rust reference for the first discrete-stream milestone.

## Implemented source surface

- `src/lib.rs` generates deterministic streams of two-frame latent units over a hidden three-unit alphabet.
- A fixed prefix guarantees each unit class appears; a seeded SplitMix64 suffix adds controlled sequence variation.
- A symbol permutation changes the visible alphabet without changing evaluator-owned unit/motif classes.
- `policy_view(end)` returns only the requested available frame prefix, without latent labels or generator seed.
- `evaluate(snapshot)` separately scores exact unit boundaries, unit equivalence classes, adjacent-pair motif boundaries and motif equivalence classes.
- The evaluator uses exact span matches before scoring pairwise class equivalence, preserving segmentation and reuse as separate measures.
- Duplicate proposals, false merging, false splitting, merged spans, invalid prefixes, invalid profiles and the maximum event bound have regression tests.

## Important limits

- This is a **single-channel discrete synthetic microcosm**, not the full multimodal ErgoStream benchmark.
- It has no active query actions, process isolation, OS sandbox, JSONL protocol, cryptographic run receipts, noise model, continuous dynamics, physical simulation or real-world data.
- `WorldV0::evaluate` is owner/evaluator-side API. Do not link it into an untrusted learner process or mistake the Rust module boundary for a security boundary.
- The candidate interface is a reference scoring contract; no learning policy is implemented.

## Verification status

- Twelve Rust unit tests passed (12 passed, 0 failed) on exact PR head `01fe2a3c3987a79611096f8e3827616500232fec`.
- The focused GitHub Actions workflow passed `cargo fmt --check`, `cargo test --locked`, and strict `cargo clippy --all-targets -- -D warnings` using Rust 1.96.0.
- Verified workflow run: [#45232056205](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45232056205), triggered by push at the exact PR head SHA.
- The authoring environment itself has no `rustc` or `cargo`; all Rust qualification above is from the remote runner.
- The repository-wide CI is a separate gate; its current status must be checked independently. Queued/skipped workflows are not passes.

## Reproduction

```sh
cargo fmt --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml -- --check
cargo test --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml
cargo clippy --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml --all-targets -- -D warnings
```