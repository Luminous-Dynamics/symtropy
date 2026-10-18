# habitat-calendar-oracle

A bounded exact reference for tiny calendar-aware scheduling cases. It models task duration as active work units, resource and task availability as half-open intervals, explicit uninterrupted vs pause/resume semantics, and scenario-specific resource blackouts.

The oracle enumerates feasible active time-slot assignments and minimizes makespan on the finite synthetic horizon. Every returned witness is rechecked by a separate validator that recomputes work totals, task/resource calendar compliance, precedence, capacity, and deadline constraints. Strict Serde types reject unknown fields. Overlapping outage intervals are rejected rather than silently merged.

## Frozen reference corpus

`docs/research/fixtures/HABITAT_CALENDAR_DISRUPTION_V0_1.json` contains nine cases and twelve scenario outcomes. The expected behaviors include:
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
