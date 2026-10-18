# habitat-calendar-oracle

A bounded exact reference for tiny calendar-aware scheduling cases. It models task duration as active work units, resource and task availability as half-open intervals, explicit uninterrupted vs pause/resume semantics, and scenario-specific resource blackouts.

The oracle enumerates feasible active time-slot assignments and minimizes makespan on the finite synthetic horizon. Every returned witness is rechecked by a separate validator that recomputes work totals, task/resource calendar compliance, precedence, capacity, and deadline constraints. Strict Serde types reject unknown fields. Overlapping outage intervals are rejected rather than silently merged.

## Frozen reference corpus

`docs/research/fixtures/HABITAT_CALENDAR_DISRUPTION_V0_1.json` contains nine cases and twelve scenario outcomes. The exact fixture bytes are pinned at SHA-256 `934ff338850ddd2a5e161b5fab10ce824e7f08d56ca8308b950a395280d3762c`. The expected behaviors include:
- pause/resume across a lunch closure: 3 work units released at 2 finish at 7, with segments [2,4) and [6,7);
- uninterrupted work on the same resource windows: earliest finish is 9, with one segment [6,9);
- a machine outage moves a two-task chain from makespan 5 to 10;
- a shutdown removes all valid uninterrupted intervals;
- a disruption can turn a schedule that misses a hard deadline into a proven-infeasible exact micro-case.

Run from the repository root:

- `cargo fmt --manifest-path tools/habitat-calendar-oracle/Cargo.toml -- --check`
- `cargo test --locked --manifest-path tools/habitat-calendar-oracle/Cargo.toml`
- `cargo run --locked --manifest-path tools/habitat-calendar-oracle/Cargo.toml -- docs/research/fixtures/HABITAT_CALENDAR_DISRUPTION_V0_1.json`

## Explicit bounds and claim ceiling

- At most 12 tasks per case and 48 integer time units per horizon.
- At most 20,000 candidate segment-sets per task and 2,000,000 expanded combination states.
- Budget exhaustion is not infeasibility or optimality.
- A proven-infeasible result applies only to the fully enumerated finite-horizon case.
- All input resources, durations, windows, and disruptions in v0.1 are synthetic test units.
- No output indicates executed work, legal approval, code compliance, permit approval, or qualified engineering evidence.

The scorecard is a functional micro-oracle, not a runtime/performance comparison. Do not extrapolate its tiny-case behavior to 500–1,000-task production schedules.


## Civil-time provenance controls

The same crate exposes `provenance::evaluate_fixture_bytes`, a metadata-only verifier for real-world calendar artifacts. It binds source revision and bytes, regional time-zone identifier, pinned time-zone database release and bytes, finite UTC expansion horizon, explicit repeated/nonexistent local-time policies, compiler/parser versions, and expanded-interval digest. Its synthetic fixture includes two accepted metadata templates and eight fail-closed negative controls.

- Contract: `docs/research/HABITAT_CIVIL_TIME_PROVENANCE_V0_1.md`
- Fixture: `docs/research/fixtures/HABITAT_CALENDAR_PROVENANCE_V0_1.json`

A provenance pass is explicitly **not** evidence that UTC expansion is correct. Actual time-zone transition conformance remains a separate follow-on requiring an exact rules-data build and independent recurrence-expansion cross-check.

Run the strict metadata/provenance negative-control suite as a JSON report:

- `cargo run --locked --manifest-path tools/habitat-calendar-oracle/Cargo.toml -- --provenance docs/research/fixtures/HABITAT_CALENDAR_PROVENANCE_V0_1.json`


## External CA-RCPSP metric-integrity audit

The crate also exposes a source-pinned audit over the 12 original CA-RCPSP instance files and the complete published result CSV, mirrored with attribution under `docs/research/external/ca-rcpsp-v1.0/`. It recomputes the longest required task duration for each instance and checks every result row labelled successful against that lower bound.

- Research note: `docs/research/HABITAT_CA_RCPSP_SOURCE_AUDIT_V0_1.md`
- Follow-up issue: https://github.com/Luminous-Dynamics/symtropy/issues/1950
- Run: `cargo run --locked --manifest-path tools/habitat-calendar-oracle/Cargo.toml -- --external-audit`

This audit deliberately preserves suspicious rows and returns a completed report with a discrepancy status. It is not a solver-ranking claim. A lower-bound finding means the reported metric/unit semantics need reconciliation before comparative claims are trusted.
