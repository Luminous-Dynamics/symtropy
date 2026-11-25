# Injector-Owned Idempotency and Linearizability v0.1

**Status:** bounded formal reference model and race-obligation catalogue. No remote injector, ROS/Gazebo, HIL, physical robot, or power-loss result is claimed.

## Why the owner ledger needs its own proof

A coordinator-side write-ahead journal cannot provide at-most-once execution if the downstream owner accepts the same key more than once. The injector owner therefore needs its own linearization point.

The essential operation is an atomic, durable create-if-absent over:

\`(idempotency\_key, payload\_digest, logical\_effect\_id, attempt\_id)\`

That reservation must commit before any non-idempotent effect call. Concurrent same-key requests cannot both acquire the reservation. A changed payload cannot overwrite the original binding.

The owner then tracks a lifecycle such as:

\`ABSENT -> RESERVED -> EFFECT_MAY_HAVE_OCCURRED -> APPLIED | NO_EFFECT | UNKNOWN\`

The critical failure cut is after the effect may have happened but before the final outcome is durably stored. The correct recovered disposition is UNKNOWN unless independently resolved. The owner must not execute the same key again. If the ledger loses its reservation across owner restart, that is loss of idempotency history—not proof of no effect.

This is intentionally not an exactly-once claim. A reservation can be durable while the effect is not yet known to have happened. The system may safely preserve at-most-once admission at the cost of an unresolved operation. A later physically evidenced NoEffect may support a separately authorized new attempt with a new attempt identity.

## Bounded model

\`model/InjectorLedgerLinearizability.tla\` has two coordinator requesters, one logical key, same-payload or changed-payload modes, an atomic-reservation toggle, owner-ledger durability, and an owner crash after an effect before outcome commit.

The safe configurations check:

- concurrent same-key/same-payload calls produce at most one reservation grant;
- concurrent same-key/different-payload calls produce at most one grant and a conflict for the losing payload;
- no more than one physical effect occurs for the key;
- a recovered Unknown outcome is not replay authority.

The unsafe controls deliberately:
1. remove the atomic recheck at reservation commit, allowing two callers that both read an empty key to grant themselves execution;
2. lose the owner ledger across restart and replay the same key after the effect may already have happened.

These controls must produce the specific reservation-alias and duplicate-effect counterexamples.

## Linearizability evidence beyond TLC

The finite model is only a schedule oracle. A real owner qualification still needs a history-based checker over concurrent invocation/response pairs and an independently counted effect. Record:

- invocation and response events with monotonic sequence IDs;
- caller, key, payload digest and attempt identity;
- owner boot/process epoch and storage transaction/commit IDs;
- the create-if-absent reservation point;
- effect invocation/return and any external device acknowledgment;
- crash/restart boundaries and read-only lookup responses.

Check histories against the owner contract. Same key/same payload returns the existing reservation/result; same key/different payload always conflicts; every effect maps to one committed reservation; no two effects share one attempt key. A lookup must not report NoEffect while an effect could still occur or may already have occurred without independent resolution.

Test concurrency with two independent processes and then multiple hosts, and inject owner crashes before reservation, after reservation, immediately before effect, after effect before outcome commit, and after outcome commit before response. Process crashes and OS/power-cut testing are separate layers.

## Benchmark arms

1. Atomic durable compare-and-set with key/payload binding.
2. Non-atomic read-then-write reservation mutant.
3. Durable reservation with fail-closed ambiguous post-effect recovery.
4. Lost owner history after restart.
5. Same key reused with a changed payload.
6. Coordinator-only journal without owner-side reservation.
7. Owner-side reservation plus coordinator write-ahead journal.

Report reservation linearization correctness, duplicate-grant rate, duplicate-effect rate, changed-payload conflict rate, ambiguous/unknown rate, false NoEffect rate, recovery latency, p50/p95/p99 reservation latency, and availability/false-hold cost separately. Never average a false safety result away.

## Claim ceiling

TLC validates only the finite abstraction and selected constants. The race fixture is a declared scenario catalogue, not executed concurrency evidence. A local fake ledger is not a real remote injector. A linearizable database reservation is not automatically atomic with a physical actuator; the actual effect boundary must enforce or honor the one-shot identity, otherwise a crash after effect can still create an ambiguous physical outcome. This work does not prove physical safety or exactly-once effects.
