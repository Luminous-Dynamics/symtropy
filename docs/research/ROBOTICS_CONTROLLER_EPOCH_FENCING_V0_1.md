# Controller Epoch Fencing v0.1

**Status:** reference implementation and research contract; exact-head tests remain unverified until CI completes.
**Scope:** in-memory controller outcome admission for simulation/reference integrations.
**Not in scope:** persistent anti-replay, authenticated transport, physical stopping, atomic multi-process control, or safety certification.

## Research shift: identity includes controller lifetime

A controller ticket alone is not a complete lifecycle identity if a native controller can restart and reuse ticket strings, request counters, or tick values. Treat effective ticket identity as:

```text
(controller_epoch, controller_ticket, request_id)
```

A monotonic event tick remains scoped to that controller epoch. Events from a prior epoch must not become current merely because the ticket string matches or its tick is larger.

This matches a real integration requirement, not just a synthetic API concern: ROS 2 Control manages controller/hardware lifecycles and supports controller restarts and error-driven deactivation/recovery. The adapter must preserve those lifecycle transitions in its own evidence stream rather than flattening them into one long-running process assumption. See the [ROS 2 Control Kilted controller-manager documentation](https://control.ros.org/kilted/doc/ros2_control/controller_manager/doc/userdoc.html) and [Rolling controller-manager documentation](https://control.ros.org/master/doc/ros2_control/controller_manager/doc/userdoc.html).

## Implemented interface

`epoch_fence.rs` adds:
- `ControllerEpoch`: a nonzero, ordered generation token.
- `EpochFencedOutcomeLedger`: globally reserves request IDs while retaining a separate `ControlOutcomeLedger` for each epoch.
- `advance_epoch`: accepts strictly increasing generations only.
- `register_submission(epoch, receipt)`: only accepts a submission from the current epoch.
- `ingest(epoch, ticket, outcome)`: checks current epoch before mutating the underlying lifecycle ledger; rejects stale epochs and current-epoch events referring to requests registered in older epochs.
- `unresolved_requests_from_prior_epochs`: exposes old requests which did not reach a terminal outcome before the epoch boundary.
- A global capacity bound across epochs. A restart does not free request IDs or silently evict old tombstones.

Native ticket strings may be reused after restart because the effective key is namespaced by epoch. Request IDs may not be reused by this object, even across epochs.

## Required supervisor contract

The epoch fence is intentionally small. Its guarantees depend on the integration supplying the epoch correctly. The controller/supervisor must:
1. Allocate a strictly increasing generation for every process/controller incarnation that can invalidate tickets or tick counters.
2. Persist or externally derive the generation so process restart, rollback, host reboot, restore-from-snapshot, or failover cannot resurrect a prior generation.
3. Publish the epoch together with every native ticket and controller outcome; never infer an event's epoch from the current request alone.
4. Establish the new epoch before it accepts outcome events from the restarted controller.
5. On a lifecycle fault, place the native controller/hardware into its platform-defined safe state independently of this Rust evidence ledger.
6. Authenticate the event source and bind epoch + ticket + request ID to the controller instance. This module does not authenticate event transport.
7. Define how the authoritative generation is allocated across multi-process failover. An in-memory counter, wall-clock timestamp, process ID, boot-time timestamp, or caller-supplied integer alone is not a durable fencing authority.

If the authoritative epoch store is unavailable, stale, or cannot prove monotonicity, the integration must not accept new outcome events as current. If it cannot establish that a controller really restarted, it must not invent a new epoch merely to make an old event pass.

## State behavior at restart

When epoch 1 advances to epoch 2:
- Epoch-1 ticket/event deliveries are rejected as stale.
- An epoch-1 request presented with epoch 2 is rejected as a prior-epoch request.
- Prior-epoch active requests remain recorded and count as unresolved; they are not silently converted to Completed, Failed, or Cancelled.
- A new epoch-2 request may use a native ticket string reused by epoch 2's controller, but must have a globally new request ID.
- Capacity remains consumed by every retained request/tombstone. Full capacity fails closed until an explicit durable compaction/rekey design exists.

This behavior prevents a stale event from completing a current request. It does **not** prove that a physical command stopped at restart. The motor/controller/hardware safety layer still owns that behavior.

## Conformance tests added

The Rust module includes focused tests for:
- nonzero initial epoch and positive capacity;
- strict epoch advancement, including same and backward generation attempts;
- valid current-epoch submission and terminal outcome;
- stale events rejected after restart without mutating previous state;
- current-epoch delivery of a prior-epoch request rejected;
- old-epoch submissions rejected;
- request ID reuse forbidden across generations;
- native ticket-string reuse correctly namespaced by epoch;
- global capacity and tombstone retention across restart;
- terminal prior-epoch requests excluded from unresolved count;
- multiple restart boundaries invalidating every previous generation.

Authored tests are not test-pass evidence; exact-head CI is required.

## Next benchmark integration

Add the epoch-fence cases to the existing [paired recovery protocol](./ROBOTICS_PAIRED_RECOVERY_BENCHMARK_V0_1.md), especially:
- restart before the first outcome;
- restart while a request is Executing;
- restart after terminal completion;
- late old-epoch event delivered after a new-epoch request reused the same ticket string;
- generation rollback and generation-store unavailability;
- event delivery after snapshot rollback/failover.

Score stale-epoch acceptance as a non-compensatory outcome-integrity violation. Report recovery performance separately. A higher task success rate cannot compensate for cross-epoch event acceptance.

## Evidence claim ceiling

This implementation is an in-memory conformance layer. It shows how the API can reject stale epochs when the caller passes the correct authoritative epoch, but it does not prove that the epoch is durable, authenticated, globally unique across failover, or tied to physical hardware state. Those claims require an integration test against the real supervisor and persistence authority, followed by HIL fault injection and platform-specific safety review.
