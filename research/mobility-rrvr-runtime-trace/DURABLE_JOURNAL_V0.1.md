# RRVR Durable Reset and Dispatch Journal v0.1

## Purpose and boundary

This module is a transport-neutral write-ahead journal for RRVR reset and fault-dispatch operations. It is designed to be buildable and testable without ROS 2 or Gazebo. It does not call a simulator, injector, driver or actuator; it cannot issue a native permit; and it cannot establish that a physical effect occurred.

The critical caller rule is:

> Do not invoke a mutating transport unless the corresponding intent append has returned successfully.

A successful append means the complete newline-terminated entry has been written and the journal file has returned successfully from `sync_all`. A failure or ambiguous result after the external call is represented as an outcome record; it is never repaired by resending the original operation.

## On-disk format

The journal is newline-delimited JSON. Every entry contains:

- schema version `0.1`;
- canonicalization identifier `serde-json-value-sorted-keys-v0.1`;
- strictly increasing sequence number starting at 1;
- previous entry SHA-256 (all-zero genesis digest for sequence 1);
- one typed reset/dispatch event;
- SHA-256 over the canonical entry payload excluding the entry digest itself.

The record hash includes the schema version, canonicalization identifier, sequence, previous hash and full event. The JSON parser rejects unknown fields. Each line is bounded to 1 MiB and replay is streaming rather than whole-file buffering.

The cross-language known-answer fixture is `fixtures/durable_journal_known_answer_v0_1.jsonl`. Implementations in another language must reproduce its exact record hash before claiming v0.1 interoperability. Hash-chain integrity detects changed bytes and broken ordering relative to the supplied log; it is not a signature, producer authentication or proof that an external device emitted the record.

## Reset lifecycle

A reset intent commits:
- reset identity;
- plan and simulator-profile digests;
- world ID, prior world epoch and clock ID;
- expected initial-state digest;
- run ID and run epoch.

The journal accepts only a new reset identity and rejects reusing the same (run, world, prior epoch) scope under a different reset ID. Reset outcomes distinguish transport response from read-only observation/reconciliation.

A verified reset requires all of:
- a nonempty new world epoch different from the prior epoch;
- the exact committed clock ID;
- the exact committed initial-state digest;
- a valid raw observation-artifact SHA-256.

An accepted transport response is not a verified reset. An unknown or unresolved outcome remains in the recovery projection as requiring read-only reconciliation. The same reset intent cannot be retried automatically. A rejected transport response is not accepted as proof of no physical effect. It remains in the recovery projection as requiring read-only state/ledger reconciliation. Only the owner-side read-only result can move it to a terminal qualified disposition.

## Dispatch lifecycle

A dispatch intent is accepted only after a verified reset and must match its plan, simulator profile, world, run, run epoch, new world epoch and clock. It includes the logical-effect/attempt identities, campaign/pair/scenario/fault identities, target, injector fault domain, schedule, parameter digest, idempotency key and payload digest.

Before the journal writes the dispatch intent, it reconstructs the same canonical request used by `gazebo_adapter::dispatch_request_fingerprint` and verifies that the payload digest matches. This prevents the journal from committing a request hash that describes a different target, fault, clock, world generation or schedule.

An idempotency key can be recorded only once. Outcomes are payload-bound. A transport response of Unknown or an unresolved ledger query can only be followed by a read-only injector-ledger lookup. Reconciliation records require a separate ledger-artifact digest. `NoDispatchProven` is terminal for that old key; it is not permission to send the same request later. A genuinely new attempt requires a new physical-attempt identity and independent RRVR recovery admission.

## Writer lease, crash behaviour and recovery

`Journal::open` obtains a single-writer lease using exclusive file creation. A stale lock left by process death is not automatically stolen. An operator must establish that the previous writer is gone and explicitly remove the stale lease. The clean-close path only unlinks the same lease inode it originally created.

Appending is write-then-sync. If writing or syncing fails, the in-process writer becomes poisoned and will refuse additional appends. On restart, replay rejects torn final lines, empty lines, oversized lines, invalid UTF-8/JSON, unknown schema/canonicalization, sequence gaps, broken previous-hash links, digest mismatch and illegal event transitions. It never truncates or repairs a damaged tail automatically.

`recovery_projection_file` and `Journal::recovery_projection` provide a deterministic read-only projection:
- resets with intent-only, response-accepted-awaiting-observation, response-unknown, response-rejected-awaiting-observation, unresolved, verified or rejected-hold status;
- dispatches with intent-only, response-unknown or response-rejected status needing injector-ledger reconciliation;
- accepted dispatches still awaiting execution/physical evidence;
- terminal no-dispatch-proven and payload-conflict states.

The projection always reports `authority_created=false` and `mutation_permitted=false`. It describes what must be reconciled; it does not perform reconciliation. The `rrvr_journal_verify` CLI verifies a quiescent journal and emits this projection without taking a writer lease or contacting any transport.

## Remaining integration work

This PR branch adds the library and tests, not the live adapter wiring. The next implementation must:
1. call `append_reset_intent` and sync before the actual Gazebo ResetSimulation request;
2. append the transport response (or unknown) and independent reset observation;
3. refuse dispatch unless the journal's verified reset binding matches the current session;
4. append `DispatchIntent` and sync before calling the real injector;
5. append the response or unknown outcome;
6. restore the verified journal projection after coordinator restart and do read-only ledger/state reconciliation before considering any further action;
7. link the journal to the authenticated execution/observer evidence bundle.

The durable journal is local history. The actual injector must still implement a durable, atomic, linearizable idempotency-key-to-payload-to-outcome ledger. A hash chain cannot make a non-durable remote injector idempotent.

## Claim ceiling

The current implementation is a file-backed reference contract with mocked tests. It has not yet been compiled or run on a CI head in this branch, and it is not wired to a live ROS 2/Gazebo transport. File and directory sync semantics must be qualified on the deployment filesystem and storage stack. No claims are made about remote injector persistence, owner authentication, observer independence, HIL, physical behaviour or robotics safety.
