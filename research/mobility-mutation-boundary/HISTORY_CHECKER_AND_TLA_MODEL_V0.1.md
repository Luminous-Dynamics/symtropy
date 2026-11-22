# Authority Mutation History Checker and TLA+ Model v0.1

**Tracking:** Symtropy #6669 / draft PR #6671  
**Evidence status:** executable Rust source and bounded TLA+ specification authored; tool runs pending exact-head workflows. No proof/test pass is claimed in this document.

## 1. Why a second verifier is needed

The application-boundary unit tests exercise selected transitions. They do not, by themselves, establish that arbitrary concurrent histories can be explained by the protocol's sequential semantics.

The history checker asks a different question:

> Given the exact invocation/response history observed by a complete recorder, is there at least one sequential ordering that preserves real-time precedence and produces every observed result under the authority/action/recovery state machine?

This is the same broad problem solved by established linearizability tools such as Porcupine and Jepsen's Knossos-based checker. CDAAB specializes the sequential model to:
- full M1/M2/platform authority binding;
- clock/boot identity and lease validity;
- global ticket one-shot consumption;
- monotonic application sequence;
- at most one unresolved physical mutation;
- explicit M1 outcome reconciliation;
- recovery/replan-permit requirement after no-effect or partial effect.

The Rust checker is not intended to replace Porcupine, Jepsen, Loom, Shuttle, TLC, Apalache or Verus. It provides a typed, dependency-free reference oracle whose semantics are specific to this project's action protocol.

## 2. History input contract

Each operation record carries:
- a unique operation ID;
- invocation and response event indices from one ordered recorder;
- typed operation input;
- the observed typed result.

The event indices are **not wall-clock timestamps**. They are monotonic sequence positions assigned by one recorder so real-time precedence can be established without comparing clocks from different devices.

For operations A and B, if response(A) is before invocation(B), A must precede B in every legal sequential ordering. Overlapping operations may be ordered either way.

The recorder must declare whether the trace is complete. Any dropped, truncated, unflushed or uncorrelated event makes the trace incomplete. The checker must not reconstruct missing invocation/response edges from approximate timestamps.

## 3. Sequential operations

The current reference model supports:

### ReplaceLiveSnapshot

Publishes a new authoritative M1/M2/platform snapshot. Epoch regression is rejected. An update does not silently clear an already pending physical mutation.

### Apply

Checks the action ticket against the current snapshot, verifies clock/boot and exclusive lease bounds, rejects replay, and refuses any new action while another application is pending.

A valid application:
- globally consumes the ticket ID;
- increments the application sequence;
- records the exact ticket binding;
- occupies the single pending-mutation slot.

The slot remains occupied regardless of whether the native callback reports accepted, definite no-effect or unknown. A definitive no-effect still enters the M1-approved recovery path; an ambiguous result cannot be treated as rejection.

### ResolvePendingAction

Consumes a result supplied by the existing independent-observer/M1 reconciliation owners:
- Satisfied: clears the pending slot and requires a fresh authority snapshot for the next action.
- NoEffect: clears the slot only with an M1 recovery-permit commitment and a strictly newer authority snapshot.
- PartialEffect: clears the slot only with an M1-approved replan/recovery permit and a strictly newer authority snapshot.
- Unknown: retains the hold; no next snapshot or permit is accepted as a substitute for physical reconciliation.

The Rust reference validates the shape of those inputs. It does not authenticate the observer, evidence digest or permit, nor does it establish physical truth.

## 4. Checker algorithm and bounds

The file src/history.rs implements a bounded backtracking checker.

1. Validate recorder completeness, operation IDs, response presence, result presence and unique event positions.
2. Derive real-time predecessor constraints from response-before-invocation order.
3. Enumerate admissible sequential orders that respect those constraints.
4. Apply each operation to the independent sequential state model.
5. Compare each generated state/result with the observed operation result.
6. Return the first valid sequential ordering, or rule out all admissible orders.

Result semantics:

| Result | Meaning |
|---|---|
| Linearizable | At least one complete sequential explanation exists |
| NonLinearizable | Every admissible ordering was explored and contradicted the specification |
| Incomplete | Recorder data is missing, a structural input is invalid, the history exceeds the bound, or the search budget is exhausted |

Current v0.1 bounds:
- maximum 20 operations per history;
- default search budget 250,000 explored candidate transitions.

The algorithm does not claim asymptotic scalability to long histories. Its first purpose is a small independent oracle for the critical mutation boundary. Larger histories should be decomposed only when the decomposition preserves dependencies and real-time ordering; Jepsen-style independent-key partitioning is not valid if authority revocation or a mission-wide pending slot couples the operations.

A bounded search that exceeds its budget returns INCOMPLETE even if no counterexample has yet been found. PASS is existential: one valid order suffices. FAIL is universal over the explored, complete bounded history.

## 5. Public adversarial history requirements

The unit tests cover:
- overlapping apply/revocation with application linearizing first;
- completed revocation before stale application;
- global ticket-ID reuse across different attempts;
- unknown application followed by a retry;
- pending action blocking a second mutation;
- unknown outcome retaining the hold;
- no-effect recovery requiring a permit;
- missing response and incomplete recorder;
- search-budget exhaustion;
- incorrect application sequence receipts.

The next conformance tranche should add generated schedules for:
- 2–4 callers;
- multiple authority changes;
- two or more tickets per active attempt;
- reordering, duplication and delayed response delivery;
- crash/restart around consume, application and receipt persistence;
- operation result loss after native application;
- attempted duplicate delivery through a second queue.

Every test must assert the native sink call count. Helper-return assertions are insufficient.

## 6. TLA+ finite-state model

formal/AuthorityMutationBoundary.tla defines the protocol as atomic state transitions. The accompanying CFG uses a finite universe of tickets, bindings, attempts, clock domains, boot epochs and recovery permits.

The TLC invariants include:
- type correctness;
- no ticket reused in the application log;
- consumed-ticket set matches application history;
- every application is bound to the declared ticket's exact binding, attempt, epoch, clock/boot and valid time interval;
- pending-slot count is at most one and exactly matches application/resolution accounting;
- application sequence agrees with history position.

The model includes explicit actions for authority replacement, clock advancement, native application, satisfied reconciliation, no-effect recovery and partial-effect replan. An unresolved/unknown action has no clearing transition except a later qualified resolution.

The workflow pins the stable TLA+ TLC 1.7.4 artifact by release URL and published SHA-1 checksum, then runs the finite model. SHA-1 here is used to check the pinned release asset against the published checksum, not as a security/signature proof. TLC v1.7.4 includes a correction for an issue where multi-worker liveness checking could miss a violation; this model checks safety invariants only.

TLA+ 1.8.0 was released as a pre-release on 19 November 2026. v0.1 deliberately pins the stable 1.7.4 rather than taking a prerelease dependency. Re-evaluate and explicitly version-bump the tool if the stable release changes.

References:
- TLA+ release page: https://github.com/tlaplus/tlaplus/releases
- TLC command-line guide: https://docs.tlapl.us/using:tlc:start
- Apalache documentation: https://apalache-mc.org/docs/apalache/index.html

## 7. Complementary concurrency and proof portfolio

Recommended sequence:

| Tool/method | Role | Evidence limitation |
|---|---|---|
| Custom CDAAB history checker | Cross-component action/authority/reconciliation semantics | Bounded histories; not production trace authentication |
| TLC | Exhaustive finite state-space exploration for this TLA+ abstraction | Only the modeled states/transitions and finite parameters |
| Loom | Exhaustive small Rust thread interleavings at the instrumented synchronization boundary | Does not reason about concurrency primitives not replaced/instrumented in the Loom model |
| Shuttle | Reproducible randomized schedules for broader stress cases | Passing randomized schedules is not an exhaustive proof |
| Apalache | SMT-backed bounded exploration and inductive-invariant attempts | Bounded/inductive result depends on model and solver assumptions |
| Verus | Selected pure Rust invariant proofs and refinement lemmas | Does not automatically prove the native controller, hardware or physical behavior |

Use each tool for its actual strengths:
- TLC/Apalache for the abstract protocol.
- The custom history checker for externally observed histories.
- Loom for exhaustive small interleavings of the Rust synchronization implementation.
- Shuttle for reproducible larger schedule exploration.
- Verus only after a stable pure transition core is extracted and its specifications can be maintained.

Do not call randomized schedules exhaustive. Do not infer that a TLA+ proof establishes that the Rust/native implementation refines the model; that correspondence needs separate tests, code review and proof obligations.

References:
- Porcupine: https://github.com/anishathalye/porcupine
- Jepsen consistency models: https://jepsen.io/consistency/models
- Loom: https://docs.rs/loom/latest/loom/
- Shuttle: https://docs.rs/shuttle/latest/shuttle/
- Verus: https://verus-lang.github.io/verus/guide/
- Apalache: https://apalache-mc.org/docs/apalache/index.html

## 8. Claim ceiling

A PASS from the Rust history checker or TLC model establishes only the corresponding bounded software-model property. It does not establish:
- completeness or authenticity of a production event recorder;
- cryptographic authenticity of authority bindings or permits;
- atomicity at a native controller/driver/hardware boundary;
- durable one-shot consumption across crash/restart;
- observer independence or physical outcome truth;
- simulator fidelity, HIL, physical safety, operational qualification or regulatory approval.

The next implementation gate is to instrument the actual native application boundary and feed its raw histories into this checker, while retaining an independent observer/evaluator for physical outcomes.
