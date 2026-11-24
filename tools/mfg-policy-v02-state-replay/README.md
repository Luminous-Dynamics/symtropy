# Manufacturing Policy v0.2 Event-Sourced State Replayer

**Package:** `mfg-policy-v02-state-replay/0.1.0`  
**Benchmark:** `MFG-POLICY-V02-CHALLENGE-001`  
**Related:** [#7189 benchmark hardness](https://github.com/Luminous-Dynamics/symtropy/issues/7189), [#7308 live-state and replay requirements](https://github.com/Luminous-Dynamics/symtropy/issues/7308), [#7279 corpus](https://github.com/Luminous-Dynamics/symtropy/pull/7279), [#7286 prefix gateway](https://github.com/Luminous-Dynamics/symtropy/pull/7286), [#7306 action admission](https://github.com/Luminous-Dynamics/symtropy/pull/7306)

## Purpose

This crate reconstructs a synthetic operational state from an ordered execution ledger and per-epoch policy decision frames. It does not consume a simulator's final state summary as truth. It independently derives operation state, resource setup/occupancy, and inspection capacity from the initial static case, visible event prefixes, and ordered transitions.

The event vocabulary is deliberately small:

- `START_ACTION`: binds a proposal to an independently recomputed admissibility check and a supplied admission receipt;
- `PROCESS_TICKS`: consumes a positive number of processing ticks on the committed resource;
- `INSPECTION_RESULT`: records the method and terminal quality disposition for an operation awaiting inspection.

The admission receipt is treated as an assertion to compare, not as the source of feasibility. Proposals and receipts both carry the SHA-256 digest of the exact serialized decision frame; the replayer recomputes the digest and requires both values to match before applying a start. It independently recomputes operation/resource eligibility, predecessor completion, qualification at the visible cutoff, resource availability/capacity, setup duration, processing duration, and inspection method/capacity.

## Invocation

```text
mfg-policy-v02-state-replay <execution-ledger.json>
```

The ledger contains one decision frame for each contiguous epoch beginning at zero. A later frame must preserve the complete earlier observable-history prefix byte-for-byte and the static case must remain identical. Transition order within an epoch is the array order and each transition has a unique ID.

## Snapshot and lineage

After every epoch the replayer emits an operational snapshot containing:
- operation status, remaining processing ticks, committed resource, and quality disposition;
- resource setup family, active operation, failure/availability state, and current-epoch capacity usage;
- inspection station capacity usage;
- exact decision-frame and visible-history SHA-256 digests;
- event-time cut, applied transition IDs, and previous-snapshot digest;
- a digest of the snapshot body excluding its own digest field.

JSON object keys are canonicalized by serde_json's ordered map representation. Arrays preserve declared execution order. The snapshot digest is an integrity checksum for deterministic replay, not a signature or proof of source authenticity.

## Fail-closed invariants

- case, benchmark, static case, frame epoch and history length bind exactly;
- decision epochs are contiguous and future information cannot be injected into an earlier frame;
- prior observable history is immutable as later prefixes arrive;
- initial operation and resource identities are unique and internally consistent;
- operation starts require a Waiting operation, completed predecessors, eligible/capable resource, Admissible qualification, available resource, free single-lane capacity and a declared setup transition;
- the supplied admission receipt must say ADMISSIBLE and exactly match the independently derived processing/setup/inspection fields;
- continuation must preserve the already committed resource;
- processing ticks cannot exceed remaining work and cannot advance an uncommitted operation;
- completion transitions to inspection-pending when inspection is required;
- inspection requires the declared applicable method and free inspection capacity;
- duplicate transition IDs, duplicate starts, double-booking, stale receipts and impossible state transitions are rejected;
- each epoch emits a deterministic snapshot linked to its predecessor.

The single-lane resource model matches the current v0.2 corpus, whose resource capacity is 0 or 1 per epoch. Active operations retain their resource commitment across epochs; a `CONTINUE` transition is allowed to use the occupied lane, whereas a new `START` cannot double-book it. Higher-capacity parallel machines, overlapping intervals, setup crews, transport resources, rework loops, multi-station inspection routing, lot splitting and continuous-time calendars are not modeled yet. The replayer rejects capacity calendars above one rather than silently approximating them.

## Validation

The focused workflow runs formatting, adversarial Rust tests, JSON-schema checks for every emitted snapshot/replay receipt, deterministic replay, and strict Clippy. Eleven tests cover a three-epoch dependent-operation trace, cross-epoch continuation, exact-frame proposal/receipt digest binding, stale/forged receipt mismatch, duplicate starts, resource double-booking, incomplete predecessor, failure-at-cutoff, future-prefix mutation, invalid processing ticks, and inspection-capacity violations.

## Claim ceiling

This is a synthetic event-sourced state-transition replay reference, not physical production replay. It does not authenticate external sensor/evidence sources, prove product conformity, establish factory savings, control machinery, establish ISO conformance, or demonstrate superiority to commercial APS. It enables the next integration step: have the trusted simulator emit the same state lineage and let the action-admission verifier consume a replay-derived snapshot.
