# Journaled Gazebo Mutator Integration v0.1

**Status:** software integration increment. No live ROS 2/Gazebo transport, remote injector durability, OS/power-cut, HIL or physical-robot qualification is claimed.

## Reset boundary

- Reset identity binds plan, simulator profile, world, run and run epoch, and stays stable across the world-epoch change caused by reset.
- The API performs read-only profile/world checks first.
- It appends and syncs the typed reset intent before calling ResetSimulation.
- Transport-return envelope and post-reset world observation are stored as distinct artifacts.
- Existing reset identity is handled through read-only observation/reconciliation; it is never sent to ResetSimulation a second time.
- preflight_transport_read_only avoids the ordinary preflight path's pause mutation during restart recovery.

## Dispatch boundary

- The journal projection is checked before dispatch.
- Existing logical-effect/attempt identity is reconciled through the owner-ledger lookup path, never through a process-local registry.
- New intent is appended and synced before injector dispatch.
- The request's attempt-bound rrvr/v0.2 key binds logical-effect ID and attempt ID in addition to plan, campaign, pair, scenario, fault, run, world, epoch and clock context.
- The world and target are rechecked after sync and immediately before the injector call.
- The exact queried payload digest and raw owner receipt bytes are mandatory before a terminal ledger reconciliation can be recorded. The compatibility key-only lookup returns no binding or owner artifact and therefore remains Unresolved.
- Adapter return envelopes are explicitly not raw wire bytes or authenticated owner receipts.

## Fresh-attempt authority boundary

This adapter is deliberately not an attempt authority. Once any dispatch intent exists for a logical-effect identity within the same plan/run scope, a caller-supplied different attempt_id is rejected with FreshAttemptAdmissionRequired. Changing the attempt ID cannot bypass old effect history.

A fresh attempt after NoEffect must be admitted by the existing native application / authority-linearized mutation owner after exact old-attempt NoEffect, persistence resolution, end-to-end downstream quiescence, current M1/M2 and live binding, and consume-once RecoveryPermit checks. The adapter cannot mint or infer this admission. This is intentionally conservative and can reduce availability until the native owner composition is connected.

The new test verifies that changing attempt_id from attempt-1 to attempt-2 does not call the injector or append a second dispatch intent.

## Test coverage added

- A simulator test double reads/verifies the journal file inside reset_simulation, proving the reset intent is visible and synced at the mutator boundary.
- Reset identity remains stable across the successful world-epoch change and read-only preflight/recovery.
- A journal-observing injector reads/verifies the dispatch intent inside its actual dispatch_once call.
- After response loss and journal reopen, the same attempt follows read-only owner lookup and injector dispatch count remains one.
- A substituted fresh attempt ID is rejected before the injector call.

## Remaining qualification

- Process termination at each write/sync/mutator/response boundary.
- File/directory sync, torn-tail, stale-lock and storage-failure campaigns.
- Real ROS 2/Gazebo transport and raw request/response evidence.
- Real injector atomic durable create-if-absent, concurrent linearizability, and owner-authenticated payload-bound receipts.
- Native owner recovery-permit composition for a fresh attempt.
- Endpoint token/epoch enforcement.
- HIL, brownout/power-cut and physical-robot evaluation.

This API does not make the journal and external effect one atomic transaction. It enforces coordinator intent-before-call ordering and conservative restart suppression. Unknown remains held; NoDispatchProven is terminal for the old key and does not authorize a retry.
