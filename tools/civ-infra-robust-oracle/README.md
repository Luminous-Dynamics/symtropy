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

- Only explicitly declared finite discrete support plus an exhaustive joint state set can support an R0 robust classification.
- Bounded intervals are not treated as safe merely because endpoints/samples are present; R0 reports unknown until an interval-safe evaluator exists.
- Unbounded unknowns, unknown evidence and incomplete scenario coverage prevent robust-feasible claims.
- Missing per-candidate state results or missing objective/constraint evidence produce unknown, not pass.
- Unknown JSON fields (including unsupported probability values) are rejected.
- Candidate and state results are sorted for semantic determinism; the input digest still changes when the exact source bytes change.
- The test-only exhaustive oracle recomputes state conjunctions independently from production classification helpers.

## Claim ceiling

Every result is synthetic and applies only to the explicitly declared finite state set. This is neither a hydraulic solver nor evidence of real-world engineering safety, validation, calibration, certification or permitting. It does not yet implement survey-action ranking or probabilistic Value of Information.
