# Energy Reconciliation Evidence Revalidation

## Status

This document defines the evidence-boundary contract for `EnergyReconciliationAudit` values produced by the core physics reconciliation layer.

`EnergyStateSnapshot::reconcile()` constructs an internally coherent audit, but `EnergyReconciliationAudit` is public and serializable. After crossing a serialization, FFI, network, plugin, cache, or manual-mutation boundary, the original constructor invariants cannot be assumed.

Strict validation consumers must therefore call the checked evidence surface before using reconciliation results in a claim, gate, adaptive-fidelity decision, or persisted evidence artifact.

## Core rule

> Reconciliation evidence must re-prove its structure and arithmetic after any mutation or serialization boundary.

A numerically small-looking residual is not trusted merely because it is present in a previously valid data structure.

## Checked surface

`EnergyReconciliationEvidenceExt` provides:

- `validate_evidence()`
- `max_abs_residual_joules_checked()`
- `unexplained_reservoir_count_checked(tolerance)`
- `fully_reconciled_checked(tolerance)`

All checked reporting methods revalidate the audit before producing an answer.

## Required evidence invariants

### Finite summaries

The following must remain finite:

- initial tracked internal energy;
- final tracked internal energy;
- net external energy;
- total closure error.

### Unique reservoir identity

Each tracked `EnergyPort` may occur at most once in the reconciliation entries.

### Stable-reservoir arithmetic

For a reservoir represented at both endpoints:

`measured_delta = final - initial`

and

`residual = measured_delta - ledger_delta`.

All represented values must be finite and the stored values must reproduce those arithmetic relationships exactly under the same `f64` operations used by the validator.

### Presence-transition shape

An appearing reservoir must have:

- `initial = None`;
- `final = Some(finite)`;
- `measured_delta = None`;
- `residual = None`.

A disappearing reservoir must have the inverse endpoint shape.

A stable reservoir must not be listed as a presence transition.

Presence-change records must be unique and must refer to a tracked reconciliation entry.

### Untracked-port integrity

`untracked_ledger_ports` must:

- contain no duplicates;
- contain no external ports;
- not duplicate a port already represented by tracked endpoint state.

### Boundary arithmetic

The stored summary must satisfy:

`total_closure_error = (final_total - initial_total) - net_external`

with finite intermediate arithmetic.

### Tolerance validity

Checked tolerance-based reporting rejects non-finite or negative tolerances. An invalid tolerance is evidence failure, not an implicit pass/fail default.

## Negative controls

At minimum, tests must prove rejection of:

1. NaN reservoir residuals;
2. inconsistent stored residual arithmetic;
3. malformed appearing/disappearing reservoir shapes;
4. presence-change metadata attached to a stable numeric reservoir;
5. inconsistent boundary closure summary;
6. duplicate/untracked/external port misuse;
7. invalid tolerances.

## Claim boundary

This layer validates the **integrity of a reconciliation artifact**. It does not make the underlying physical reservoir model exact.

A checked, fully reconciled audit means:

- the represented endpoint reservoirs are structurally coherent;
- their measured changes agree with the causal ledger within the declared tolerance;
- no unresolved reservoir lifecycle transition or untracked internal ledger port remains;
- the serialized evidence itself has not become numerically or structurally invalid.

It does not prove that every physically relevant form of energy exists in the current model, nor does it upgrade approximations such as the current scalar-mean rotational inertia path.

## Relationship to the energy authority contract

This is the final evidence-integrity layer for the physical channel defined in `docs/ENERGY_AUTHORITY_CONTRACT.md`.

Heuristic/semantic and operational-budget quantities are not admitted to this reconciliation merely because they use joule-like names. They require a validated physical conversion and an explicit typed ledger transfer first.
