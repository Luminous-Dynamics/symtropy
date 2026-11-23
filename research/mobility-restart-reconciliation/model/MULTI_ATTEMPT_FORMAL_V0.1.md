# RRVR multi-attempt formal lifecycle v0.1

**Tracking:** [#6985 — bounded multi-cycle refinement and differential conformance](https://github.com/Luminous-Dynamics/symtropy/issues/6985)  
**Model:** RestartSafeEffectContinuityMultiAttempt.tla  
**Safe configuration:** RestartSafeEffectContinuityMultiAttempt.cfg  
**Unsafe control:** RestartSafeEffectContinuityMultiAttemptUnsafe.cfg  
**Rust reference contract:** research/mobility-rrvr-runtime-trace/src/attempt_lineage.rs in PR #6983

## Purpose

The original RestartSafeEffectContinuity model is intentionally pinned and remains unchanged. It covers one original attempt and one recovery admission. The separate v0.1 model extends the lifecycle to a finite sequence of up to three attempts under one stable logical-effect identity.

The extension exists to answer a question that the one-recovery model cannot answer: after attempt 2 itself crashes or may have caused a physical effect, can the system prove what happened before admitting attempt 3?

## Model structure

The bounded state uses an indexed attempt map over 1..MaxAttempts. Every attempt has:
- one stable logical-effect ID and an immutable ordinal/parent relation;
- its own authorization boot/controller epoch pair;
- reservation, dispatch and native acceptance state;
- its own command-pending and command-owner state;
- separate physical outcome and durable-persistence state;
- separate physical-outcome and downstream-quiescence observer qualifications;
- an explicit common-cause and subject-self-report fault flag;
- an independent mission-resolution state;
- per-attempt controller-queue and downstream-execution fence epochs;
- captured admission facts for the native boundary.

Global state retains one active attempt, boot/controller epoch, one-shot recovery permit target/use count, fresh ticket identity/epochs, M1/M2 and live-binding validity, physical mutation count, process state and crash-cut label.

### Recovery edge

A child attempt can be reserved only after the preceding attempt has reached reconciled. Reconciliation requires:
1. physical outcome no_effect;
2. qualified physical-outcome observer proof;
3. qualified downstream-quiescence observer proof;
4. the two observer proofs are independently qualified under the declared fault abstraction;
5. mission resolution no_effect;
6. persistence is resolved as committed or definitely_not_committed, with no in-flight store call;
7. controller queue and downstream execution graph have separately fenced the exact current boot/controller epoch;
8. there is no pending old command;
9. M1/M2 remain current.

The permit is issued for exactly the next attempt. A fresh ticket binds that child to the current boot/controller epoch. Native acceptance checks the current ticket, permit, M1/M2 and live binding at admission, then consumes the permit. The native boundary remains the authority owner; RRVR model state does not mint an independent action capability.

### Repeated recovery

The same sequence is repeated for the next edge:
- attempt 1: crash/restart, reconcile NoEffect, admit attempt 2;
- attempt 2: crash after native acceptance and before an effect, or after a possible effect; restart, independently reconcile attempt 2 to NoEffect, admit attempt 3;
- attempt 3: may apply the physical effect once and terminate the logical effect.

The model permits an Applied terminal attempt, but its outcome cannot have a descendant. Partial, Unknown, Disputed and NotObserved outcomes are not retryable.

## Invariants

| Invariant | Meaning |
|---|---|
| TypeOK | Every state field remains in its declared finite domain |
| NoDuplicatePhysicalMutations | At most one Applied or Partial physical mutation is counted for the logical effect |
| AttemptIdentityIsStable | Every attempt belongs to the same logical-effect identity |
| AttemptParentLinksAreStable | Attempt 1 has no parent; each later attempt names its immediate predecessor |
| ReservationPrecedesAcceptance | An accepted/reconciled attempt has a durable reservation and dispatch predecessor |
| RetryAdmissionIsQualified | Every child has a reconciled parent with all exact NoEffect/persistence/observer/fence conditions |
| RecoveryAdmissionWasCurrentAndFenced | The child captured fresh boot/controller, fences, M1/M2 and binding at admission |
| NoRetryAfterUnsafePhysicalOutcome | Applied, Partial, Unknown or Disputed cannot parent a retry |
| NoRetryAfterAmbiguousPersistence | InFlight/IndeterminateCommit cannot parent a retry |
| NoRetryAfterUnqualifiedObserver | Missing or non-independent observer qualification cannot parent a retry |
| OneShotPermitUseBound | Permit use cannot exceed the number of possible recovery edges |
| PermitTargetsOnlyNextAttempt | An outstanding permit can target only the next unaccepted child |
| OldQueueFenceClosesPendingCommand | Current downstream fence implies the attempt has no pending command |
| DownstreamFenceRequiresControllerFence | Downstream quiescence cannot precede/mismatch controller queue fencing |
| StoreVerdictMatchesDurableRecord | Durable record truth and acknowledged persistence verdict agree |
| NoObserverSelfQualification | Common-cause or subject-self-report faults cannot create qualified observer evidence |
| NoCommandOwnedByWrongAttempt | A pending command must retain the exact attempt owner |

## Positive and negative controls

The safe CFG uses:
- MaxAttempts = 3;
- MaxEpoch = 3;
- UnsafeRetryEnabled = FALSE.

TLC must exhaust this finite state space without invariant violations.

The unsafe CFG sets UnsafeRetryEnabled = TRUE. It enables a deliberately unsafe retry-bypass action after one physical mutation. TLC must produce a counterexample to NoDuplicatePhysicalMutations. The workflow accepts that run only when the expected invariant is named in the counterexample; a parser/tool failure is not counted as a successful negative control.

## Rust correspondence

The Rust lineage validator in PR #6983 checks a concrete evidence recording. The TLA+ model checks the bounded lifecycle abstraction. Their shared semantic fields are:
- stable logical-effect and unique attempt identity;
- immediate parent link and ordinal;
- boot/controller authorization epoch;
- persistence outcome;
- physical outcome;
- mission resolution;
- separate controller and downstream fences targeted at the child's exact epoch;
- physical observer and downstream observer qualification under the fault model;
- a one-shot next-attempt permit.

The model currently verifies the transition-level safety invariants; the exact trace-to-Rust decision/reason-code differential harness remains a separate acceptance item in #6985. No claim of complete Rust/TLA+ refinement is made until that comparison exists.

## Claim ceiling

A passing TLC run proves only the invariants of this exact finite abstraction at MaxAttempts=3 and MaxEpoch=3. It does not prove unbounded recovery, authenticate real owner receipts, establish observer physical independence outside the declared abstraction, prove the actual downstream hardware inventory is complete, or establish simulator/HIL/physical robot safety. The existing native adapter must continue to revalidate live bindings and atomically consume its own one-shot permit at its existing linearization point.
