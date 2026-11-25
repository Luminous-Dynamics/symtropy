# Journaled Gazebo Mutator Integration v0.1

**Status:** implementation increment and software-test target. Live ROS 2/Gazebo transport, remote injector durability, process power-loss, HIL and physical safety are not qualified by this document.

## Purpose

This increment connects the previously separate durable journal and plan-bound Gazebo adapter at their mutator boundaries. The new API is under gazebo_adapter::journaled; it does not change the old reference API's signature.

## Reset contract

1. Compute a stable reset identity from plan, simulator profile, world, run and run epoch. The ID deliberately does not depend on the currently observed world epoch, so a coordinator restart after a successful reset can locate the original intent rather than create a new reset identity.
2. Confirm the simulator profile and current paused world state using read-only calls.
3. Append and sync ResetIntentRecord.
4. Only then call ResetSimulation.
5. Record an adapter-level response envelope separately from the post-reset world snapshot artifact.
6. On ambiguous response or restart, use reconcile_reset / recover_reset_from_journal, which perform read-only world queries. Never repeat ResetSimulation for an existing reset identity.

preflight_transport_read_only is provided for restart recovery. It refuses a playing simulator rather than pausing it as a hidden mutation.

## Dispatch contract

1. Verify the compiled plan and a journal-verified reset.
2. If the exact logical-effect/attempt identity already exists, perform payload-bound owner-ledger lookup only. The process-local DispatchAttemptRegistry is not used as recovery authority.
3. A new operation is moved to its exact simulator cut point and target binding before its dispatch intent is prepared.
4. The idempotency token is upgraded to rrvr/v0.2 and binds the original plan/campaign/pair/scenario/fault/run/world/clock identity plus logical_effect_id and attempt_id.
5. Append and sync the typed DispatchIntentRecord.
6. Recheck the world and target after the sync, then invoke the injector exactly once.
7. Persist the normalized adapter response envelope. Unknown/rejected/ambiguous outcomes require read-only owner lookup.
8. Only an owner lookup that echoes the exact requested payload digest and supplies owner-produced raw receipt bytes can produce a terminal reconciled verdict. A legacy key-only lookup remains Unresolved.
9. NoDispatchProven is terminal for the old key; it does not authorize a retry. A fresh physical attempt requires a fresh attempt identity and the existing separately owned recovery/authority admission.

## Evidence boundary

The current GazeboSimulationTransport and GazeboFaultInjectorTransport traits expose typed return values, not raw middleware wire frames or authenticated owner receipts. The reset/dispatch response artifacts created by this adapter are therefore labelled **adapter return envelopes**, not raw transport packets. A production injector must override lookup_dispatch_bound and return:

- the digest of the exact payload queried;
- the owner’s current bound payload digest for a conflict;
- its exact owner-produced receipt bytes;
- an existing dispatch ID when it reports Accepted.

The compatibility default delegates to the old key-only lookup method and intentionally returns no payload binding or receipt artifact. That can only remain Unresolved.

The artifact store must durably store the exact bytes and return their SHA-256; the adapter verifies the digest. Hashes do not authenticate the producer or prove physical truth.

## Tests added

- A simulator test double reads and verifies the journal file inside reset_simulation, demonstrating that the reset intent is already synced before the mutator is called.
- Reset identity stability across the world-epoch change and coordinator preflight reconstruction.
- Dispatch response loss followed by journal reopen; the same logical attempt causes owner lookup, while the injector dispatch-call count remains one.

## Remaining qualification

- Process termination at every write/sync/mutator/response boundary.
- File and directory sync semantics, torn-tail, stale writer lock and storage failure campaigns.
- Real ROS 2/Gazebo transport integration and exact raw request/response capture.
- Real injector-side atomic, durable create-if-absent and concurrent linearizability testing.
- Owner receipt authentication/provenance and payload-binding conformance.
- Endpoint-side one-shot tokens and authority fencing.
- HIL, brownout/power-cut and physical robot evaluation.

This API increment does not make the journal and external physical effect one atomic transaction. It makes the coordinator's intent-before-call ordering explicit and restart-safe, while preserving Unknown when the external effect cannot be resolved.
