# Spatial Resilience Qualifier

Standalone Rust CLI and test library for the v0.1 synthetic spatial-resilience benchmark fixtures.

## Scope

The tool checks the frozen JSON manifest against its Draft 2020-12 JSON Schema, then enforces reviewed semantic invariants that a schema alone cannot express: stable sequential case IDs, unique scenario dimensions, an exact registered-invariant vocabulary, fail-closed negative-control expectations and corpus version/count.

It also includes a deliberately small, hand-checkable event oracle for one narrow temporal rule: a service restoration is not *stable recovery* until the service has remained continuously available for the declared stability window. Duplicate receipts with identical identity, generation and payload are idempotent; conflicting reuse of an event identity is rejected; simultaneous events need explicit unique sequence numbers.

**This is not a building-physics solver.** It does not compute indoor temperatures, energy use, ventilation, water quality, accessibility or safety. Passing its tests means only that the declared fixture contracts and the oracle's finite-state semantics behaved as tested.

## Commands

From the repository root:

```sh
cargo test --manifest-path tools/spatial-resilience-qualifier/Cargo.toml
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --check
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --check --json
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --oracle-check --json
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --outcome-check --json
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --decision-impact-check --json
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --validate path/to/manifest.json path/to/schema.json --json
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --oracle-validate path/to/timeline-corpus.json path/to/timeline-schema.json --json
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --outcome-validate path/to/outcome-corpus.json path/to/outcome-schema.json --json
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --decision-impact-validate path/to/decision-impact-corpus.json path/to/decision-impact-schema.json --json
```

The JSON receipt includes exact SHA-256 hashes of the manifest and schema, case and invariant counts, a pass/fail bit, stable failure codes and JSON instance paths. Exit codes are 0 for a passing validation, 1 for invalid inputs/contracts and 2 for CLI or file-read failures.

The separate machine-readable timeline oracle corpus at `docs/research/spatial-design/SPATIAL_RESILIENCE_TIMELINE_ORACLE_V0_1.json` contains eight expected event-sequence outcomes and its own Draft 2020-12 schema. `--oracle-check` executes these fixtures through the finite-state oracle and emits a content-hashed JSON receipt; `--oracle-validate` applies the same process to an external corpus.

The paired-outcome corpus at `docs/research/spatial-design/SPATIAL_RESILIENCE_PAIRED_OUTCOMES_V0_1.json` has its own schema and five known-answer comparisons. `--outcome-check` enforces common trace identity/revision, preserves each resident/task service floor independently, excludes stale or out-of-domain evidence, applies the declared uncertainty limit before ranking, and computes a Pareto frontier over service-hours (maximize) and synthetic energy, repair time, and cost units (minimize). Exact ties are retained as distinct alternatives; a weighted sum cannot rescue an infeasible candidate. All numeric values in that corpus are synthetic mechanics fixtures, not physical predictions.

The decision-impact corpus at `docs/research/spatial-design/SPATIAL_RESILIENCE_DECISION_IMPACT_V0_1.json` contains eight finite-state worlds. It validates each measurement action's source revision, evidence state, unknowns resolved and a mutually exclusive/exhaustive outcome partition; then recomputes robust-feasible and Pareto sets for each outcome. The output reports a decision-impact vector and a declared lexicographic action ordering (critical-unknown resolution, hard-feasibility change, Pareto change, worst-case residual-state count, cost, duration and action ID), with no probability or expected-VoI estimate. The corpus tests when a targeted survey should outrank a costly scan and when high-volume measurement has no decision impact.

## Negative controls in the authored test suite

- duplicate case ID and duplicate scenario dimension;
- attempted invariant-registry drift;
- a negative control mutated to a non-rejecting disposition;
- malformed JSON;
- recovery interrupted by a recurrent fault exactly at the proposed stability boundary;
- recovery restarted after a recurrence;
- identical duplicate-event delivery versus conflicting duplicate identity;
- ambiguous same-tick event ordering;
- a recovery window that extends past the horizon;
- zero-length stability window and event beyond the declared horizon.

38 unit and negative-control tests are authored but have not been compiled or executed in the editing environment. CI results must be interpreted at the exact commit: queued is not pass.

## Versioning and trust boundaries

The validator intentionally freezes the v0.1 case count, IDs, invariant registry and dimensions. A benchmark change should update the manifest, schema, validator and negative controls together, with an explicit version change. A generic schema-valid JSON document is not by itself an admitted engineering result. Physical claims still require qualified inputs, appropriate externally validated models, uncertainty and scope receipts, independent checks, and responsible professional review.
