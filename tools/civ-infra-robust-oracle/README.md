# CIV-INFRA Robust Oracle v0.1

A small, dependency-locked Rust verifier for finite **synthetic** civil-corridor worlds. It is deliberately separate from the full Bevy workspace and can run without SWMM, GPU access, field data or a long integration suite.

## Run

```sh
cargo fmt --manifest-path tools/civ-infra-robust-oracle/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civ-infra-robust-oracle/Cargo.toml
cargo run --locked --manifest-path tools/civ-infra-robust-oracle/Cargo.toml -- tools/civ-infra-robust-oracle/fixtures/robust-v0.1.json
```

The report hashes the exact fixture bytes and states the claim ceiling. The five synthetic alternatives span robustly feasible, robustly infeasible, and conditionally feasible outcomes. The robust Pareto set is expected to contain B-resilient and D-high-margin: one is shorter/has less earthwork, the other has more hydraulic margin.

## Fail-closed behavior

- R0 independently checks that the declared exhaustive state set contains every tuple in the full Cartesian product of the finite discrete supports, with no missing or duplicate tuples.
- R0 caps exhaustive Cartesian evaluation at 100,000 joint states; oversized products fail closed instead of being sampled.
- Correlated/joint supports are not yet certified by R0. A trusted boolean is not enough: correlated scenarios need a future explicit, evidence-backed dependency contract.
- Bounded intervals are not treated as safe merely because endpoints/samples are present; R0 reports unknown until an interval-safe evaluator exists.
- Unbounded unknowns, unknown evidence and incomplete scenario coverage prevent robust-feasible claims.
- Missing per-candidate state results or missing objective/constraint evidence produce unknown, not pass.
- Unknown JSON fields (including unsupported probability values) are rejected.
- Candidate and state results are sorted for semantic determinism; the input digest still changes when the exact source bytes change.
- The test-only exhaustive oracle recomputes state conjunctions independently from production classification helpers, and a separate pairwise oracle checks robust Pareto membership.
- A deterministic generated differential test evaluates 625 finite fixture variants (12,500 candidate-state evaluations) and checks classifications and Pareto membership against the test oracles. When CI sets `CIV_INFRA_CORPUS_OUT`, the test writes a machine-readable manifest with the base-fixture digest, generator seed/algorithm, each generated input digest, per-candidate oracle/evaluated dispositions, and oracle/evaluated Pareto sets. The workflow uploads that manifest as a 14-day artifact. Without the environment variable, local test runs do not write a file.

## Claim ceiling

Every result is synthetic and applies only to the explicitly declared finite state set. This is neither a hydraulic solver nor evidence of real-world engineering safety, validation, calibration, certification or permitting. It does not yet implement survey-action ranking or probabilistic Value of Information.
