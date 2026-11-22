# Mobility Native Application Adapter v0.1

**Tracking:** [MOBILITY-NATIVE-APPLICATION-ADAPTER-001 #6749](https://github.com/Luminous-Dynamics/symtropy/issues/6749)  
**Parent:** [authority-linearized mutation #6669](https://github.com/Luminous-Dynamics/symtropy/issues/6669)  
**Status:** Reference composition only. No production controller, simulator, HIL or physical platform is wired by this crate.

## Core change

This adapter composes three existing owners at one synchronous application seam:

1. **M2 action evidence** (mobility-action-evidence): revalidates the complete mission/action, platform configuration, ODD/mode, authority, lease, navigation, frame, source and freshness requirements against a fresh owner-provided live context.
2. **Authority mutation boundary** (mobility-mutation-boundary): compares the M2 ticket commitment with an independently supplied current binding commitment, checks the exact active action-attempt and clock/boot binding, consumes the ticket once, and holds the one unresolved mutation slot.
3. **Robotics bridge queue** (EpochCommandQueue): checks the controller epoch and queue lease at its synchronous callback.

Only after all three agree does the callback reach the supplied native sink. The sink must be the actual native acceptance operation, not an enqueue into another unfenced queue.

## Paradigm shift

Move from **three separately valid checks** to one **composed proof-carrying application point**.

An action is not a timeless command. It is a bounded commitment over a complete mission/platform/authority/ODD/navigation context, a controller epoch, and separately qualified time bases. The exact binding that passed planning must still be current at native acceptance.

The two clock bases are intentionally separate:
- M2 action ticket uses its declared qualified source clock and boot epoch.
- The existing robotics queue uses its own declared monotonic nanosecond clock and boot epoch.

The adapter does not convert, cast or compare those values. It requires the queue's clock identity to match before the queue compares its nanosecond lease. It independently requires the M2 live snapshot, M2 qualified instant and M2 ticket to agree.

## Provider and sink contracts

LiveContextProvider.with_live_snapshot must hold the authoritative context stable through the entire callback. An implementation that returns a cloned snapshot and releases its lock before the callback does not satisfy the contract. The caller must serialize updates to platform configuration, authority, ODD/mode, navigation, M2 clock and binding commitment against this callback.

live_binding_commitment must be independently computed by the upstream owner from the current complete binding. It must not simply copy the proposed M2 ticket commitment. The adapter compares them through the existing authority boundary and does not compute or authenticate either value.

NativeApplicationSink.apply must synchronously perform the actual native acceptance. It must not forward the payload into another command queue that can outlive the fence. The sink reports one of the existing native outcomes:
- accepted by native controller;
- definitive no-effect rejection;
- unknown after possible application.

All three outcomes occupy the existing M1/M2 unresolved-mutation hold until qualified reconciliation. Unknown is never a retry grant.

## Serial mutation rule

This reference intentionally uses capacity one, matching the current M1 single-unresolved-mutation policy. It does not enable the concurrent batch path from #6699. resolve_pending_action only clears the hold after an upstream M1/independent-observer owner has qualified the outcome and a fresh M2-valid next-action ticket is supplied under a strictly newer controller epoch.

Unknown retains the hold. No cancellation, retry, release or scheduler authority is added.

## Tests and exact-owner CI

The targeted workflow checks out exact M2 and authority-boundary owner commits and consumes the robotics bridge module from the branch containing this PR. It runs rustfmt, Rust tests and strict Clippy.

The initial test suite covers:
- positive control reaches the mock native sink exactly once;
- configuration changed after enqueue is rejected at application with zero sink calls;
- live whole-binding commitment mismatch is rejected by the existing authority boundary;
- controller epoch mismatch is rejected with zero sink calls;
- M2 clock/boot mismatch is rejected with zero sink calls;
- queue clock/boot mismatch does not pop queued work or call the sink;
- queue lease expiry at application never invokes the callback;
- unknown-after-possible-application consumes the ticket and retains the mutation hold;
- definitive no-effect still retains the hold and requires the existing recovery path.

No test here authenticates the upstream owner, proves the commitment's canonicalization, establishes physical effect, or tests a real native driver.

## Non-claims

This is a software reference composition over supplied owner snapshots and a synchronous mock/application callback. It does not establish cross-process atomicity, restart-safe anti-replay, durable epoch persistence, external authority serialization, driver/firmware queue fencing, physical effect, emergency-stop behavior, real-time deadlines, HIL, physical safety or regulatory qualification.