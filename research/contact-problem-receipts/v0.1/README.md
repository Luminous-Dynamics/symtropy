# Contact Problem Comparability Receipts v0.1

Dependency-free validator for frictional-contact benchmark input equivalence and frontend coverage.

## Two panels; do not merge their denominators

### Fixed contact-problem solver panel

All numerical methods must receive the same model law, mass properties, Jacobian, contact frames, stable contact identities, friction/material, free-velocity RHS, regularization, time discretization and solver-row digest. The validator permits solver implementation/build/configuration and outcome to differ; those are what the tournament measures.

A different contact count is not the only mismatch. Equal counts with different contact IDs, frames or row digests are not the same problem. Dropped rows, truncation or capacity overflow disqualify the comparison.

### Full engine pipeline panel

Each engine may construct contacts differently, but each receipt must account for the same frozen scene, eligible broadphase pairs, narrowphase queries, contact set and solver rows. The pair, query and actual contact denominators remain distinct.

If an independent contact oracle is unavailable, contact completeness is unavailable. A fully accounted scene/task report may still be emitted, but it is not a contact-qualified solver comparison. Equal generated/oracle counts do not substitute for canonical contact-set digest equality. A different constitutive-law digest is surfaced explicitly: task outcomes may be compared, but solver-law equivalence is not claimed.

A fixed-problem receipt distinguishes input eligibility from performance eligibility. Different solver builds and solver configurations are expected; different complete mathematical inputs are not. Capacity settings may differ when both runs admit the same complete row set. Wall-clock comparisons additionally require matching environment digests and timing observations at or above timer resolution.

A full-pipeline scene/task comparison requires complete pair/query accounting and no dropped/overflowed solver rows. A *performance-comparable* full-pipeline pair additionally requires identical independent pair-oracle population digests, identical independently qualified contact-set digests, the same constitutive-law digest, matching environment and usable timings. Until there is a separately qualified contact-equivalence mapper, differing contact sets remain task-only comparisons.

## Trust boundary

- SHA-256 values are supplied by upstream canonical serializers; this crate validates lowercase hex and compares immutable values. It does not compute hashes.
- The crate checks count conservation and declared digest equality. It does not prove the upstream oracle or authenticate telemetry.
- Missing residuals/timings remain absent; they are never synthesized as zero.
- Initialization failure, unsupported, iteration limit and numerical failure remain distinct outcomes.

## Run

```sh
cargo fmt --manifest-path research/contact-problem-receipts/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/contact-problem-receipts/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/contact-problem-receipts/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Promotion targets

Zero eligible fixed-problem comparisons with mismatched fingerprints or row digests; zero accepted dropped/overflowed rows; 100% accounting of declared pair/query/contact/solver-row denominators; unavailable contact oracles explicitly unavailable; full-pipeline cross-law outcomes labelled as different constitutive problems.

This is benchmark receipt infrastructure, not a collision detector, contact solver or performance result.
