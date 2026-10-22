# Thermodynamic Tick Transactions and Replayable Receipts

**Status:** Research RFC / candidate architecture  
**Scope:** Symtropy's thermodynamic simulation and energy accounting  
**Date:** 2026-10-16  
**Related:** [THERMO-TICK-002 #1719](https://github.com/Luminous-Dynamics/symtropy/issues/1719), [phase ordering #824](https://github.com/Luminous-Dynamics/symtropy/issues/824), [finalization idempotency #763](https://github.com/Luminous-Dynamics/symtropy/issues/763), [ledger boundary #777](https://github.com/Luminous-Dynamics/symtropy/issues/777), [admission guard PR #1713](https://github.com/Luminous-Dynamics/symtropy/pull/1713)

## 1. Thesis

A fixed-timestep scheduler is a timing mechanism, not a transaction protocol. The system should make tick identity, phase order, accepted transfers, and finalization explicit data with machine-checkable invariants.

The target is not to claim global physical conservation by bookkeeping fiat. The target is a simulation whose consequential updates can be inspected, replayed, independently recomputed, and rejected when their preconditions are not satisfied.

## 2. Contract

A tick is identified by a monotonic integer allocated at the simulation boundary. `Time<Fixed>::elapsed()`, timestep, frame index, and wall-clock time are metadata; they are not the durable transaction identity.

Candidate lifecycle:

```text
Idle
  -> Begun(tick_id, input_digest)
  -> PhysicsApplied(tick_id, physics_digest)
  -> Finalized(tick_id, receipt)
```

Every transition validates the expected state and tick ID before mutating authoritative state. Out-of-order transitions return typed errors. The first implementation should not pretend to roll back mutations it cannot stage or reverse.

### Phase requirements

- **Begin:** capture input tape identity and stable pre-state digest; reject duplicate or stale tick IDs.
- **Physics applied:** record that the expected physics phase completed for the same tick ID; phase order is enforced with Bevy system sets and explicit ordering.
- **Stage:** collect typed candidate deltas without mutating the authoritative ledger/reservoir state where practical.
- **Finalize:** validate all deltas, commit once, and emit a receipt. A repeated finalize returns the existing receipt and does not reapply consequences.
- **Failure:** produce a typed failure outcome. If a failure occurs after irreversible mutation, surface partial completion explicitly rather than claiming atomic rollback.

## 3. Receipt schema (proposal)

The first schema should be versioned and minimal:

- schema version;
- tick ID and input digest;
- previous committed state digest;
- phase completion bitmap / typed phase outcomes;
- accepted energy transfers by category and stable event ID;
- rejected event IDs with typed reasons;
- resulting state digest;
- finalization outcome.

A cryptographic hash chain may be added after the canonical encoding is defined. Hashes establish tamper-evident linkage, not correctness: the state digest must be independently recomputable from the receipt's inputs and accepted deltas.

## 4. Typed flows and system boundary

Do not conflate consumption, dissipation, regeneration, storage change, and external input/sink. A complete accounting identity requires an explicit system boundary and all relevant terms. Until those exist, a local residual must be labelled a diagnostic, not global conservation error.

Candidate sign convention:

`delta_storage = external_inputs + regeneration_into_boundary - dissipation - external_sinks - other_exports`

This is only a template: each modeled subsystem must document whether a transfer is internal (cancels at the combined boundary) or crosses the boundary. Never infer a sign from a method name or silently record regeneration as negative dissipation.

## 5. Determinism and replay

Persist a compact, versioned input tape containing initial state, tick IDs, stable entity IDs, commands/events, timestep policy, and any random seeds. Replay against a clean simulation instance. Compare per-tick receipts and state digests, not only final aggregate energy.

Define the reproducibility promise before implementation:
- same build, target, features, and input tape should be deterministic;
- cross-architecture bitwise equality is a separate, stricter goal and may require constrained floating-point behavior;
- tolerance-based numeric equivalence must specify units, normalization, and absolute/relative thresholds;
- a digest mismatch should emit a minimal first-divergence report.

## 6. Benchmark and acceptance matrix

All values below are proposed qualification targets, not measured results.

| Gate | Proposed acceptance |
|---|---|
| Transition safety | 100% rejection of illegal, stale, duplicate, and out-of-order phase transitions |
| Idempotent finalization | Duplicate finalize returns byte-identical receipt and zero additional authoritative mutations |
| Failure injection | Inject failure after each phase; outcome is either uncommitted state or an explicit partial-commit record |
| Deterministic replay | 1,000,000 ticks replayed twice on same target with identical per-tick receipt digests |
| Independent recomputation | 100% receipt/state-digest agreement with a separate verifier implementation |
| Hostile flow corpus | 100% expected rejection for negative/non-finite values, overflow, duplicate event IDs, capacity mismatch, missing categories, and invalid phase order |
| Scaling | Report p50/p95/p99 tick latency and replay throughput at 1k / 10k / 100k entities |
| Allocation | Report allocations per tick and receipt bytes; establish a baseline before imposing a budget |
| Overhead | Measure baseline versus transactional mode; do not adopt a fixed percentage gate until representative workloads exist |
| Conservation claims | No global-conservation label until all storage deltas and boundary inputs/sinks are represented and independently checked |

Report exact commit SHA, compiler/toolchain, target triple, enabled features, seed, initial state digest, benchmark workload, and raw output for every result.

## 7. Implementation plan

1. Add typed `TickId`, phase state, and typed transition errors without changing existing physics behavior.
2. Add pure state-machine tests, including duplicate and stale transitions.
3. Add canonical receipt encoding and independent receipt verifier.
4. Add a compact deterministic input tape and first-divergence replay tool.
5. Introduce staged typed energy deltas and exactly-once commit.
6. Add explicit Bevy system sets/order for begin, physics, finalize; test schedule integration rather than only calling methods directly.
7. Measure baseline and transactional overhead.
8. Only after replay semantics are stable, evaluate event sourcing, parallel deterministic reduction, and optional receipt hash chaining.

## 8. Non-goals and failure modes

- A passing unit test is not evidence of global physical conservation.
- A hash chain is not a proof that the physics is correct.
- A timestamp guard is not a full transaction state machine.
- A receipt is not authoritative if its input, event IDs, or state transition can be forged by untrusted callers.
- Do not weaken negative controls to make the replay benchmark pass.
- Do not describe unexecuted tests as passed.

## 9. Decision requested

Implement phases 1–3 first in a small Rust-only change. Keep the RFC and tests in the public Symtropy repository. Preserve compatibility adapters where needed, but make new code use typed tick IDs and receipts.