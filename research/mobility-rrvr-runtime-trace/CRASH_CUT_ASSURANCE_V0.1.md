# Journaled Mutator Crash-Cut Assurance v0.1

**Status:** research contract and bounded formal model. Nothing in this document represents a live Gazebo, ROS 2, remote injector, HIL, physical-robot, or power-cut result.

## Purpose

This increment formalizes one narrow question from RRVR issue #7361:

> Once a reset/dispatch intent is durable, can a coordinator restart cause the same physical-effect attempt to be sent again, or cause a transport response to be mistaken for physical NoEffect?

The model complements the Gazebo reset lifecycle model and the durable journal reference implementation. It does not replace either, and it does not establish Rust/TLA refinement.

## State model

The model keeps distinct state for:

- coordinator process incarnation (coordinatorUp, bootEpoch);
- durable local intent and outcome (journalIntent, journalOutcome);
- remote owner ledger (ownerLedger, ownerHistoryLost);
- in-flight response (requestInFlight, transportResult);
- mutation-call count and physical-effect count;
- the boot epoch that issued the last mutation call;
- post-restart recovery gate (fresh, blocked, reconciled).

The safe behavior is:

1. Persist the exact operation intent before the mutator call.
2. Permit at most one dispatch call for this attempt in the initial coordinator incarnation.
3. On restart, an existing intent places the adapter in blocked.
4. Reconcile through the owner ledger using a read-only lookup.
5. Map an unavailable/lost owner history to unknown, never no_dispatch_proven.
6. Keep the old attempt closed after reconciliation. A genuinely new attempt belongs to the higher-level authority/recovery process and must use a new identity.

The model treats the remote owner operation as an atomic state transition. OwnerLedgerDurable=TRUE means its key-to-payload-to-outcome state survives coordinator restart. Setting it to false models loss of the owner's previous result; a safe coordinator then remains Unknown. It does not assume that a local hash chain creates remote idempotency.

## Invariants

- NoMutationBeforeDurableIntent: no dispatch call exists without a persisted intent.
- AtMostOneDispatchCallPerAttempt: the adapter does not resend an existing attempt.
- AtMostOnePhysicalEffectPerAttempt: no duplicate modeled effect occurs.
- NoFalseNoEffect: no_dispatch_proven is impossible when the owner says Applied or when owner history was lost.
- NoMutationAfterRestartForOldIntent: the old identity cannot be mutated from a later coordinator epoch.
- NoDispatchWhileRecoveryBlocked: a blocked recovery gate cannot call the mutator.

## Mandatory negative controls

The UnsafeRedispatch mutant enables a coordinator to resend the old identity after restart. It is checked with owner-ledger durability disabled so the model can expose the duplicate-effect path. The mutant must violate AtMostOneDispatchCallPerAttempt and/or AtMostOnePhysicalEffectPerAttempt.

The UnsafeInferNoEffectFromRejectedResponse mutant turns a rejected transport response into no_dispatch_proven. The model must find a trace where the owner had already Applied the operation and violate NoFalseNoEffect.

These are mutation tests of the abstraction, not measurements of a real injector.

## Crash-cut mapping

The JSON fixture crash_cut_matrix_v0_1.json declares cut obligations for:

- before intent append;
- partial intent write or uncertain sync;
- after durable intent and before the call;
- after remote effect but before acknowledgement;
- after acknowledgement but before outcome persistence;
- rejected transport response despite possible effect;
- coordinator restart with owner history present or lost;
- crash during read-only reconciliation;
- stale writer lease or torn journal tail;
- concurrent same-key/same-payload and same-key/different-payload owner-ledger histories;
- delayed old responses after a newer process incarnation.

The fixture is an obligation catalogue, not proof that these executions have been run. Process termination, injected filesystem failures, OS crash, and physical power removal are separate test classes and must have separate reports.

## Experimental design

For every cut, record:

1. frozen campaign and attempt identity;
2. source revision, build/toolchain, storage and simulator/injector revisions;
3. declared cut and whether it was actually reached;
4. mutator calls and independently counted effects;
5. owner lookup result and source artifact digest;
6. recovered journal state and projection;
7. expected-versus-observed oracle decision;
8. elapsed time to journal verification, rehydration and reconciliation;
9. failure witness and minimized reproduction.

Use separate populations and denominators for:
- crash-cut coverage;
- valid/invalid evidence yield;
- false redispatch;
- false NoEffect;
- false recovery admission;
- false holds and unresolved outcomes;
- reconciliation availability/completeness;
- append/sync and replay latency.

Do not collapse these into one score. An unsafe false-confirmation must remain visible even when task success is high. A conservative hold must remain visible even when it is safer than an incorrect admission.

## Matched comparisons

Retain the existing protocol's baselines: native/no-overlay, ROS ros2_fault_injection, EAST-compatible validity-aware campaigns, coordinator-only restart handling, RRVR without journal, RRVR with local journal but no durable owner ledger, RRVR with durable owner ledger, naive retry, no independent observer, observer-ID counting without common-cause analysis, and removed plan/world/clock binding.

Add storage arms only under matched durability semantics:
- RRVR append-only file journal with its exact sync policy;
- SQLite WAL with synchronous=FULL, and a declared checkpoint policy;
- optionally, a mature embedded transactional store under equally explicit durability semantics.

SQLite's official documentation says WAL + synchronous=NORMAL can lose the most recent committed transactions on power loss, whereas FULL syncs the WAL on each commit. Do not claim a custom journal is faster or safer without same-device, same-filesystem, same-durability comparisons.

## Statistical methodology

Use scenario-clustered inference when repeated trials share world, simulator, seed, task, policy, hardware, injector or fault-schedule ancestry. Keep paired campaign arms together in each resample. Preregister primary endpoints and report confidence intervals for every primary comparison.

The rule-of-three approximation (3/n) applies only to independent representative zero-failure trials. It is not valid to count correlated episodes as independent evidence. Stratify by fault family, crash point, storage, owner implementation, simulator, platform and environment. Report the effective independent cluster count and exclusions.

## Claim ceiling

A TLC pass establishes invariants only for the finite abstraction and constants checked. The fixture validates the declared obligation catalogue only. A process-kill test does not establish power-loss durability. A file journal does not establish remote injector linearizability. A simulator result does not establish HIL or physical safety. A confirmed evidence verdict is not execution authority.

The next code increment remains issue #7361: wire the existing Journal immediately before the actual reset/dispatch mutators, rehydrate suppression from verified journal history, and qualify the injector-owner ledger separately.


## Model boundary note

The bounded TLA model explores one logical-effect attempt and enables coordinator crash only after the durable intent exists. It checks that an applied effect followed by loss of a non-durable owner ledger cannot be safely redispatched. Pre-intent write/sync failures, concurrent coordinator histories, reset-state transitions, owner-ledger linearizability, and physical storage fault injection remain obligation-catalogue or implementation work tracked by #7361; they are not silently represented as explored TLA states.
