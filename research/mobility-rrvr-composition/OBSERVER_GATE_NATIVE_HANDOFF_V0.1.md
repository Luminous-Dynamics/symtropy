# RRVR observer-bound native handoff v0.1

**Scope:** composition-only handoff specification for the existing Symtropy native application adapter #6749.  
**Tracking:** [MOBILITY-RRVR-NATIVE-ADAPTER-INTEGRATION-001 #6874](https://github.com/Luminous-Dynamics/symtropy/issues/6874).  
**Related:** [observer-bound composition PR #6882](https://github.com/Luminous-Dynamics/symtropy/pull/6882).

## 1. Boundary ownership

The new RRVR wrapper ends at:

`ObserverBoundDisposition::ReadyForOwnerNativeRevalidation`

It does not mint:
- `m2::ActionTicket`;
- `boundary::RecoveryPermitReference`;
- a controller epoch or queue lease;
- a native application capability;
- a dispatch token.

The existing #6749 `NativeApplicationGate` / `resolve_pending_action` remains the sole native resolution owner. It owns synchronous callback revalidation, the current M2/live whole-binding check, controller queue epoch/lease, and its call into the existing authority-linearized mutation owner.

A successful reference assessment is not a bypass around any of those checks.

## 2. Required join before calling the owner

The orchestrator must preserve four separate evidence planes for the exact old attempt:

1. **Durable persistence:** Mycelix's authoritative commit classifier and mutation/attempt ancestry.
2. **Physical effect:** independent exact-attempt outcome plus matching M1 mission resolution.
3. **Execution path:** controller queue fence and downstream graph receipts, with separate graph-quiescence verdict and fault-model-qualified observer quorum.
4. **Current authority:** fresh M1/M2 ticket, boot/controller epoch, live whole-binding commitment, and owner-issued recovery permit.

The reference crates check only the supplied packet shapes, identities, declared scopes and bounded rules. Cryptographic verification, current authority and durable consume-once state remain with their respective owners.

## 3. Pre-native decision map

| RRVR observer-bound result | Safe orchestrator behavior | Native authority implication |
|---|---|---|
| `ReadyForOwnerNativeRevalidation` | The exact old attempt is NoEffect, persistence is resolved, the declared downstream graph is quiescence-evidenced, and both observer models meet their declared quorum. | May ask the existing owner boundary to resolve the exact NoEffect recovery using the authenticated owner permit and fresh ticket. The owner revalidates all live state and consumes the permit. |
| `PhysicalEffectAlreadyObservedNoRetry` | Preserve the old Applied fact; do not schedule a second attempt for the same logical effect. | Do not invoke a NoEffect reattempt path. A separate mission progression action requires its own current authority. |
| `PhysicalEffectObservedPersistenceUnresolvedNoRetry` | Preserve the independently supported Applied fact while keeping the persistence reconciliation open. | No reattempt. Reconcile the original mutation identity with the durable owner. |
| `Hold(...)` | Preserve the exact hold reason and its component evidence. Unknown, Partial, Disputed, Refuted and InsufficientEvidence remain closed. | No fresh-action ticket/permit is passed to a NoEffect recovery call. Any unresolved result remains held by the existing owner. |

The table is a control-flow contract, not a new implementation of the native owner API. Exact enum variants, signatures and resolution semantics must be imported from the pinned #6749 and authority-owner revisions during #6874 implementation.

## 4. Race and fault requirements for the eventual adapter integration

The application-boundary implementation must test:
- live M1/M2 or whole-binding change after RRVR assessment but immediately before callback;
- permit replay, cross-attempt substitution, cross-boot replay and duplicate resolution;
- controller epoch change between pre-assessment and callback;
- late old-attempt completion after downstream quiescence was first claimed;
- a hidden or newly added command-holding graph node;
- crash after permit consumption but before next physical effect;
- Applied + IndeterminateCommit and NoEffect + IndeterminateCommit as separate outcomes;
- Unknown, Partial, Disputed, Refuted and InsufficientEvidence refusing new action authority;
- no lower driver/firmware queue left outside the declared graph.

For each invalid/held case, the exact native sink must receive zero calls. For the valid NoEffect case, the owner boundary must demonstrate exactly one permit consumption under the exact current ticket and epoch.

## 5. Non-claims

This note does not claim #6749 has been integrated with RRVR. It does not implement a native adapter, authenticate a permit, perform a queue fence, or establish physical NoEffect. That work remains explicitly tracked by #6874 and requires source-level integration against the exact pinned owner crates and their conformance workflows.
