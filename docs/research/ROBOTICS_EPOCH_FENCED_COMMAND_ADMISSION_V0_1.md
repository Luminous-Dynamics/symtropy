# Epoch-Fenced Controller Command Admission v0.1

**Status:** in-process reference wrapper and mock tests; no native ROS 2/hardware command path is integrated.
**Parent:** #2874 (persistent epoch authority contract).

## The boundary that matters

A read such as `ensure_current(epoch)` is not sufficient to fence a command. Another owner may advance the controller generation after the read and before the command is enqueued. The native command admission point must validate the generation atomically with accepting the command, or a single owner must serialize both operations. The generation must be carried with the returned native ticket and retained in outcome registration.

## Implemented in `epoch_fenced_sink.rs`

- `EpochFencedControllerSink<S>` requires an explicit epoch on every submit call and intentionally does not implement the untagged `ControllerSink` trait.
- It checks the supplied generation against the in-process current generation before calling the inner sink.
- Stale submissions are rejected without calling the inner sink.
- The accepted receipt returns `EpochFencedControllerReceipt { epoch, receipt }`, making `(epoch, ticket, request_id)` the effective identity.
- The generation can advance only strictly upward.
- Six focused tests cover current-generation receipt binding, zero sink calls for stale submissions, ticket-string reuse across generations, non-monotonic advance rejection, controller submission failure, and mutex-owned serialization.

The wrapper's `&mut self` API prevents simultaneous mutation of the same instance under Rust's borrowing rules. If shared across threads, callers must wrap the *whole wrapper* in one mutex and hold the lock across epoch advancement or submit. This only serializes one process.

## What it does not guarantee

- It does not query the persistent authority itself. The trusted supervisor must obtain the epoch from `ControllerEpochAuthority` and activate it here as part of controller restart/reinitialization.
- It does not stop, cancel, or drain commands accepted under an older epoch. The native controller/hardware layer must clear or safely terminate old work during restart, using its existing watchdog/e-stop/safe-state behavior.
- It cannot fence another process that submits directly to the native controller or advances a different authority. Production hardware/IPC admission must atomically validate the token at the actual enqueue/application boundary.
- It is not a ROS 2 controller plugin, a physical safety function, or a proof of collision avoidance.

## Benchmark requirement

Keep stale command admissions distinct from stale outcome events. The recovery scorer now counts `stale_epoch_command_admission_acceptances` separately and treats any such acceptance as a non-compensatory hard-gate failure. A task-success or recovery-time improvement cannot compensate for the wrong generation reaching the command sink.

Freeze cases that deliver a stale command before and after a controller restart, reuse the same controller ticket in the new generation, race an old submission against epoch advancement, and attempt direct/unfenced submission. The oracle must check both the return decision and sink call count. For hardware qualification, repeat these cases against the actual native admission layer and prove whether already-accepted prior-generation work is cancelled, drained, or bounded by an independent watchdog.

## Industry context

ROS 2 Control separates controller lifecycle management from hardware interfaces and supports controller error-driven deactivation and fallback behavior; the controller manager also has a real-time update path. Epoch-store access should therefore happen during generation activation and the fencing token should be propagated to a bounded command admission interface, rather than adding blocking persistence queries to every high-rate control update. The actual sink still needs atomic fencing semantics. References: [ROS 2 Control Kilted controller manager](https://control.ros.org/kilted/doc/ros2_control/controller_manager/doc/userdoc.html), [Rolling architecture](https://control.ros.org/master/doc/ros2_control/doc/getting_started/getting_started.html).
