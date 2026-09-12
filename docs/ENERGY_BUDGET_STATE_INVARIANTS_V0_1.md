# EnergyBudget state invariants v0.1

## Status

Implemented source contract only. This document is not runtime qualification.

This tranche implements the constructor/state-validation decision tracked by #767 on top of the accepted-transfer semantics in #784.

## Goal

`EnergyBudget` is persistent thermodynamic state. It must never rely on every caller independently remembering which floating-point combinations are physically meaningful.

The v0.1 contract separates three cases:

1. **trusted/static construction**;
2. **historical/persisted state validation**;
3. **live mutation**.

Those cases deliberately have different failure semantics.

## Persistent invariants

A valid reservoir requires:

- `max_energy` finite and `>= 0`;
- `available` finite and within `[0, max_energy]`;
- `temperature` finite and `> 0 K`;
- `entropy` finite and `>= 0`;
- `heat_capacity` finite and `> 0`;
- `consumed_this_tick` finite and `>= 0`;
- `regenerated_this_tick` finite and `>= 0`;
- `lifetime_consumed` finite and `>= 0`;
- `collapsed` consistent with the declared usable-energy threshold.

The current usable-energy threshold is `1e-10 J`.

## Constructor policy

### `try_new(max_energy)`

This is the explicit validated constructor.

- positive finite capacity => full, active reservoir;
- zero capacity => valid zero-energy reservoir, born collapsed;
- negative capacity => error;
- NaN/+Inf/-Inf capacity => error.

No invalid floating-point state is constructed.

### `new(max_energy)`

`new` is retained for source compatibility with existing trusted/static callers.

For a valid capacity it is equivalent to `try_new`.

For an invalid capacity it fails closed to a **zero-capacity collapsed reservoir** rather than preserving the invalid scalar. This is a compatibility safety behavior, not a persistence repair mechanism. New code that needs to detect configuration errors should use `try_new`.

## Persistence/network validation

`EnergyBudget::validate()` checks the complete invariant set and returns a typed `EnergyBudgetStateError`.

Validation is observational only:

- it does not clamp;
- it does not normalize;
- it does not flip collapse state;
- it does not rewrite historical values.

A corrupted save/network state can therefore be quarantined or rejected while preserving evidence of the corruption.

## Live mutation rule

`consume`, `regenerate`, and `dissipate_heat` refuse to mutate an already-invalid reservoir.

Each mutator computes all resulting values first and commits only when the resulting state remains finite and bounded. A failed command therefore does not partially update energy, entropy, lifetime counters, or telemetry.

`tick_reset` remains permitted to clear per-tick telemetry counters even when another persistent field is invalid. That does not repair the persistent reservoir and is useful before quarantine/reporting.

## Derived-value rule

Derived control values fail closed when state is invalid:

- `available_work()` => `0`;
- `fraction_remaining()` => `0`;
- `net_flow_this_tick()` => `0`;
- `has_energy()` => `false`;
- `is_collapsed()` => `true`.

Call `validate()` when software must distinguish true depletion from corrupted state.

This prevents NaN/Inf reservoir corruption from becoming motor authority, HUD throughput, or extractable-work authority.

## Collapse consistency

When a live consumption update leaves less than or equal to the usable-energy threshold, the remaining numerical residue is normalized to exactly zero and `collapsed = true` in the same transaction.

Regeneration can recover a valid collapsed reservoir. The collapse flag is cleared only once the accepted transfer raises usable energy above the threshold.

Zero-capacity reservoirs therefore remain collapsed permanently unless a later API explicitly changes capacity; v0.1 does not add capacity mutation.

## Public fields remain staged

The invariant-bearing fields remain public in v0.1 because making them private is a much larger source-compatibility change.

That means external code can still deliberately construct corruption by direct field mutation. The important v0.1 improvements are:

- public constructors no longer create invalid state;
- persistence has an explicit validation theorem;
- checked live mutators refuse invalid state;
- derived authority values fail closed.

A later compatibility-measured tranche may move fields behind getters/setters or a validated deserialization type.

## Interaction with other thermodynamic work

- #765 hardens mutator inputs and actual regeneration telemetry.
- #784 / #776 makes finite sources debit only accepted transfer.
- #757 / #770 consumes extractable work for locomotion.
- #783 / #763 closes the fixed-tick thermodynamic interval after physics.
- #777 still needs to define the ledger control volume before `conservation_error` can be interpreted as physical closure.

This tranche does not solve the separate contact-local dissipation attribution issue discovered during #777 review.

## Source-level theorems

Tests are included for:

- valid positive construction;
- explicit zero-capacity collapse;
- fallible rejection of negative/NaN/Inf capacity;
- fail-closed compatibility construction;
- corruption reporting without repair;
- collapse mismatch detection;
- invalid-state rejection by live mutators;
- tiny residual energy collapsing to exact zero;
- invalid-state derived values failing closed;
- existing consumption/regeneration/thermal accounting behavior under valid state.

## Required qualification

Before merge readiness, execute on the exact PR head:

- `cargo fmt --check`;
- `cargo test -p symtropy-consciousness-physics`;
- `cargo clippy -p symtropy-consciousness-physics --all-targets -- -D warnings` or the repository-equivalent exact CI surface;
- launcher/default workspace build;
- launcher build with `--features consciousness-runtime`;
- regression of #784 accepted-transfer tests after the constructor/state change;
- regression of #757 extractable-work semantics when this stack is later rebased into that line;
- corrupted-state persistence fixture proving validation reports rather than rewrites state.

No exact-head runtime PASS is claimed by this document.
