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
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --adaptive-policy-check --json
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --validate path/to/manifest.json path/to/schema.json --json
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --oracle-validate path/to/timeline-corpus.json path/to/timeline-schema.json --json
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --outcome-validate path/to/outcome-corpus.json path/to/outcome-schema.json --json
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --decision-impact-validate path/to/decision-impact-corpus.json path/to/decision-impact-schema.json --json
cargo run --manifest-path tools/spatial-resilience-qualifier/Cargo.toml -- --adaptive-policy-validate path/to/adaptive-policy-corpus.json path/to/adaptive-policy-schema.json --json
```

The JSON receipt includes exact SHA-256 hashes of the manifest and schema, case and invariant counts, a pass/fail bit, stable failure codes and JSON instance paths. Exit codes are 0 for a passing validation, 1 for invalid inputs/contracts and 2 for CLI or file-read failures.

The separate machine-readable timeline oracle corpus at `docs/research/spatial-design/SPATIAL_RESILIENCE_TIMELINE_ORACLE_V0_1.json` contains eight expected event-sequence outcomes and its own Draft 2020-12 schema. `--oracle-check` executes these fixtures through the finite-state oracle and emits a content-hashed JSON receipt; `--oracle-validate` applies the same process to an external corpus.

The paired-outcome corpus at `docs/research/spatial-design/SPATIAL_RESILIENCE_PAIRED_OUTCOMES_V0_1.json` has its own schema and five known-answer comparisons. `--outcome-check` enforces common trace identity/revision, preserves each resident/task service floor independently, excludes stale or out-of-domain evidence, applies the declared uncertainty limit before ranking, and computes a Pareto frontier over service-hours (maximize) and synthetic energy, repair time, and cost units (minimize). Exact ties are retained as distinct alternatives; a weighted sum cannot rescue an infeasible candidate. All numeric values in that corpus are synthetic mechanics fixtures, not physical predictions.

The decision-impact corpus at `docs/research/spatial-design/SPATIAL_RESILIENCE_DECISION_IMPACT_V0_1.json` contains eight finite-state worlds and eleven measurement actions. Each action carries a current source revision, evidence state, explicit cost/duration basis IDs, synthetic cost/time units, unknowns resolved and a mutually exclusive/exhaustive outcome partition. The evaluator recomputes robust-feasible and Pareto sets for each outcome. It reports a decision-impact vector and applies a declared lexicographic ordering (critical-unknown resolution, hard-feasibility change, Pareto change, worst-case residual-state count, cost, duration and action ID) **only to actions that change the robust-feasible set, Pareto set, or declared critical-blocker set**. Actions that change none of these remain visible but are not recommended. A staged action that clears one blocker while another remains is flagged as blocker progress, never as design feasibility; no probability or expected-VoI estimate is emitted. The corpus tests when a targeted survey should outrank a costly scan and when high-volume measurement has no decision impact.

The Rust differential test also generates 100 deterministic finite decision worlds with 100 candidates each and checks the robust-feasible/Pareto outputs against a separate brute-force reference across full and reduced state sets (at least 10,000 candidate-state evaluations; the current generator covers 35,000). This is correctness stress coverage, not a runtime-performance claim.

The adaptive-policy corpus at `docs/research/spatial-design/SPATIAL_RESILIENCE_ADAPTIVE_POLICIES_V0_1.json` contains bounded policy trees. It checks conditional follow-up actions, child outcome partitions restricted to the states that survive the parent branch, stopping only when the robust/Pareto decision is stable and no critical blocker remains, explicit escalation/reopen leaves, per-path cumulative cost/duration and worst-case budgets. Its known answers compare a two-step adaptive survey (worst-case cost 5 synthetic units, duration 3 ticks) with a universal scan (cost 10, duration 4) under the same terminal obligations; it also rejects a child branch for an outcome made unreachable by its parent. These are toy-oracle mechanics, not real cost-savings evidence.

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
- zero-length stability window and event beyond the declared horizon;
- adaptive policy: stop with unresolved blocker, stale source revision, incomplete action partition, cycle, unreachable outcome, missing branch, depth-budget overrun and state-dependent terminal decision.

57 unit and negative-control tests are authored. The prior 44-test version passed at exact commit `87398c530fa67e9ada48d768a60145c4e3d3cc05`; the seven adaptive-policy tests are new and must pass on the updated exact head before the expanded suite is considered qualified. The editing environment has no Cargo/rustc/rustfmt, so CI results must be interpreted at the exact commit: queued is not pass.

## Versioning and trust boundaries

The validator intentionally freezes the v0.1 case count, IDs, invariant registry and dimensions. A benchmark change should update the manifest, schema, validator and negative controls together, with an explicit version change. A generic schema-valid JSON document is not by itself an admitted engineering result. Physical claims still require qualified inputs, appropriate externally validated models, uncertainty and scope receipts, independent checks, and responsible professional review.
