# RRVR durable dispatch and blinded observation v0.1

**Issue:** [#7148 — crash-safe dispatch journaling and blinded observation](https://github.com/Luminous-Dynamics/symtropy/issues/7148)  
**Journal:** src/dispatch_journal.rs  
**Gazebo integration entry point:** dispatch_fault_with_write_ahead in src/gazebo_adapter.rs  
**Execution-evidence validator:** src/execution_evidence.rs  
**Campaign plan:** [CAMPAIGN_PLAN_V0.1.md](CAMPAIGN_PLAN_V0.1.md)

## Why this increment matters

The adapter previously used an in-memory set to suppress duplicate dispatch calls. That is insufficient across process death. A restarted process has forgotten the in-memory set and could repeat a request whose outcome is unknown.

A second gap was circular observation: a fault observer must not receive the expected signature it is supposed to verify. Passing the expected value into the observer makes it possible for a defective observer to echo the answer.

A third gap was conflating a successful step-control response with proof of the simulator's new world iteration.

v0.1 addresses all three with separate evidence boundaries.

## 1. Durable write-ahead dispatch

The file-backed journal is JSONL, append-only, strictly parsed and hash-chained. Each record contains:
- monotonically increasing sequence number;
- previous record SHA-256;
- idempotency key;
- canonical request SHA-256;
- phase and optional outcome/dispatch ID;
- record SHA-256 over the canonical record body.

The legal phase path for one key is:

    IntentPersisted -> ResponseRecorded
    IntentPersisted -> ReconciliationRecorded
    ResponseUnknown -> ReconciliationRecorded -> ReconciliationRecorded ...

The first record (IntentPersisted) is written and fsynced before the external injector call. The parent directory is synced when the journal or lock file is newly created. A torn final line, blank interior line, invalid JSON, duplicate sequence/intent, broken previous-hash link, changed request digest or illegal state transition fails closed.

A single-writer lease is established using an exclusive lock file. Normal orderly shutdown removes the lease marker. A process crash leaves the marker; this implementation intentionally does not remove a stale lock automatically. An operator must establish that the previous process cannot still be writing before removing it and reopening the journal. This sacrifices automatic availability to avoid two concurrent writers.

### Crash semantics

| Crash/failure point | Durable local state | Restart behavior |
|---|---|---|
| Before intent sync completes | Intent not established; no dispatch is called | Journal recovery/validation is required before another attempt |
| After intent sync, before dispatch | Intent exists | Suppress dispatch and query the same injector ledger key |
| After dispatch, before response record sync | Intent exists, response unknown | Suppress dispatch and query the same injector ledger key |
| After response record sync | Intent + response exist | Return recorded result; never dispatch the same key again |
| During journal append | Torn tail or valid complete record | Reject torn tail; never truncate/retry automatically |

The local journal does not itself provide remote exactly-once execution. It provides durable local intent and suppresses a second dispatch after restart. The injector must implement its own durable idempotency ledger, ideally with linearizable lookup/dispatch semantics. If it cannot, the outcome remains Unknown and recovery stays blocked.

### Reconciliation

Reconciliation is read-only. It calls lookup by the same idempotency key and records Accepted, Rejected, NoDispatchProven or Unresolved. An Unresolved lookup may be repeated later, because it does not mutate the remote system. A resolved outcome is terminal for that key. NoDispatchProven is not permission to reuse the same key; a new attempt must be independently authorized and receive a new key under the higher-level recovery contract.

## 2. Blinded independent observer

The Gazebo observer interface receives:
- exact target ID and fault ID;
- world epoch and simulation clock ID;
- requested observation iteration.

It does not receive the expected fault-signature digest. It returns its independently measured signature, observer ID/revision/fault domain, raw artifact ID/digest, world epoch, clock, iteration and simulation time. Only after that evidence exists does the adapter compare the measured digest with the signature committed in the plan.

The platform plan pins observer ID, revision, binary digest and fault domain. The campaign validator rejects an observer whose identity or declared fault domain is the same as the injector's. The execution-evidence verifier additionally requires each observation receipt to come from that exact pinned observer build.

This prevents direct answer-echoing; it does not establish universal physical independence. Common power, clock, host, calibration and measurement dependencies still need a separate fault-model inventory and physical qualification.

## 3. Simulator control versus simulator observation

Gazebo ROS 2 Simulation Interfaces provide services/actions for reset, step, state query/update and simulator feature discovery. The adapter uses these for control, but does not equate a successful StepSimulation response with proof that the world advanced as requested.

Each world snapshot now includes a separately captured WorldStatistics receipt from the pinned world stats topic. The adapter cross-checks:
- exact topic and message type;
- simulator producer/build identity;
- raw artifact ID and SHA-256;
- simulation clock ID;
- iterations against the requested simulation step;
- sim_time against the state snapshot and execution receipt.

Gazebo documents the world stats topic as carrying sim_time, real_time, real_time_factor and iterations. The current server documentation lists the WorldStatistics topic as throttled to 5 Hz. Consequently, a concrete live transport must wait for a fresh stats sample with a bounded observation timeout; it must not use wall-clock sleeps to advance physics or reuse a stale sample as proof. Timeout or mismatch aborts the cut-point qualification. See [Gazebo SDF world statistics](https://gazebosim.org/docs/jetty/sdf_worlds/) and [Gazebo Server topic API](https://gazebosim.org/api/sim/8/classgz_1_1sim_1_1Server.html).

The execution bundle now carries one world-statistics artifact per fault event and verifies that it agrees with the event's planned iteration and simulation time. A dispatch receipt without that independent telemetry remains incomplete.

## 4. Compatibility and test-profile scope

The v0.1 campaign validator accepts the currently documented LTS combinations:
- ROS 2 Jazzy + Gazebo Harmonic (Ubuntu Noble 24.04);
- ROS 2 Lyrical + Gazebo Jetty (Ubuntu Resolute 26.04).

The reference fixture uses the Jazzy/Harmonic labels. It remains a synthetic contract fixture: simulator/plugin/observer/injector identifiers and binary hashes are placeholders, not measurements from an installed live stack. A real qualification must capture the exact package versions, binary hashes, server config, SceneBroadcaster build, world profile and stats topic from the target container. See [Gazebo's ROS installation matrix](https://gazebosim.org/docs/latest/ros_installation/).

## 5. Tests and claim ceiling

The Rust suite contains tests for:
- intent is synced before dispatch;
- duplicate dispatch suppressed after reopen;
- crash after intent and after external dispatch reconciles without redispatch;
- intent write failure prevents external dispatch;
- response write failure leaves intent and returns Unknown;
- request mutation under the same key is rejected;
- journal tamper, broken chain, torn tail, duplicate writer and missing intent reject;
- unresolved lookups can be repeated and later resolved without dispatch;
- blinded observer accepts only the pinned observer/domain and measured signature;
- Gazebo world-statistics mismatch and unreviewed ROS/Gazebo pairings reject.

A passing journal test proves deterministic local record semantics and tests the mocked transport path. It does not prove remote idempotency implementation, live telemetry freshness, real observer independence, live ROS/Gazebo operation, HIL, physical fault delivery, native authority or robot safety. Those remain separate gates.
