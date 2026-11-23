# Solver Coverage Contract v0.1

A dependency-free Rust research contract for **what a solver was asked to compute, what it actually completed, and what evidence it returned**.

This is a standalone research crate, intentionally outside the production Cargo workspace. It does not execute physics, modify solver state, depend on Bevy, or grant engineering authority.

## Why

A produced numeric zero is not the same thing as a zero returned because a contact row was dropped, an island was skipped, or an observable is unsupported. Value-only readback can hide missing computation.

The contract keeps two independent questions separate:

1. **Protocol completeness:** did every catalogued observable receive exactly one valid disposition, and does every declared row family have internally consistent accounting?
2. **Required-work completion:** were all required requested observables produced and all required solver rows solved without drops, skips, unsupported rows, non-convergence or aborts?

An explicitly reported omission can be protocol-complete while the physical computation remains incomplete. That is intentional. Aggregate-only row counts cannot attribute a produced value to one of several partially omitted rows; the report marks this as incomplete row attribution. If every requested row in a linked family was omitted, a row-linked observable cannot be marked Produced.

## Contract

- Every run binds a scenario, engine build, solver profile, capability census, run ID and request-set digest.
- Every observable has a stable entity/component key, declared unit/frame, requested/required state and optional solver row-family association.
- Every catalogued key has exactly one result.
- `Produced` requires a finite value, producer identity and explicit uncertainty semantics. A value of exactly zero is valid.
- `NotRequested` is permitted only for catalogued entries marked unrequested.
- Requested non-produced statuses require a reason and cannot carry a numeric value:
  `Unsupported`, `CapacityDropped`, `SleepingSkipped`, `NotConverged`, `Invalid`.
- `Missing` is a protocol failure. Missing records, duplicate keys, unexpected keys, malformed digests, and invalid accounting are rejected.
- Per-family row accounting is exact:
  - requested = admitted + sleeping-skipped + capacity-dropped + unsupported;
  - admitted = solved + not-converged + execution-aborted;
  - reserved <= configured capacity and admitted <= reserved.

## Canonical identities

The crate exposes deterministic, order-independent canonical preimage bytes for the request and complete receipt. The SHA-256 operation is deliberately external: the caller hashes the request preimage and supplies the computed digest to `validate_receipt_with_computed_request_digest`, which compares it to `request_set_sha256`. `validate_receipt` alone checks the structure and digest syntax; it does not authenticate caller-supplied digests.

## Tests

The unit suite includes:
- computed exact zero;
- capacity-dropped result;
- sleeping-island skip;
- missing and duplicate dispositions;
- unrequested catalog entry;
- explicitly unsupported optional torsion output;
- capacity and count-accounting failures;
- aggregate row-attribution ambiguity under partial omission;
- canonical order invariance;
- malformed digest and caller-computed request-digest mismatch;
- invalid uncertainty interval.

Run:

```sh
cargo fmt --manifest-path research/solver-coverage-contract/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/solver-coverage-contract/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/solver-coverage-contract/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Integration boundary

This is the independent semantic validator for [#7030](https://github.com/Luminous-Dynamics/symtropy/issues/7030). It is not yet wired to live solver counters or the run-health receipt. That runtime integration remains behind the prerequisite and trace-noninterference sequence in #1472–#1475.

It does not prove physical truth, contact-law completeness, solver correctness, independent sensor accuracy, model suitability, performance superiority or actuation authority.
