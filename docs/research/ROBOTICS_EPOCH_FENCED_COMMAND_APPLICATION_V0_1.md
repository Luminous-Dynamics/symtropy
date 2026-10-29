# Epoch-Fenced Command Application v0.1

**Status:** bounded in-process reference queue and deterministic unit tests; no ROS 2, IPC, simulator command queue, or physical controller is wired to this module.

## Why this is the next boundary

Submission-time fencing alone does not cancel commands that have already entered a queue. A controller restart can leave an old-generation command waiting behind the new controller unless the adapter explicitly drains, cancels, or fences queued work. A ticket receipt is not proof that a physical action was applied, and an epoch check followed by deferred application has a time-of-check/time-of-use gap.

## Implemented reference semantics

`EpochCommandQueue<T>` adds:

- Explicit controller epoch, positive bounded capacity, and a monotonic deadline attached to every queued command.
- Rejects commands that are already expired at enqueue and rechecks the deadline immediately before application using the same declared monotonic clock domain.
- Fail-closed rejection of stale-epoch enqueues, duplicate request IDs while already pending in the same epoch, and queue overflow. Overflow never evicts an older accepted command to make room for a newer one.
- Monotonic epoch advancement that returns every queued prior-generation command as a cancellation/audit record and empties the queue.
- A single `apply_next` callback instead of a public pop operation, keeping the same in-process queue exclusively borrowed while the application callback executes.
- Counters for accepted/rejected enqueues, capacity pressure, queue cancellation at epoch transition, and defensive stale drops.
- Deterministic tests for queue capacity, stale and duplicate inputs, epoch rollback, transition purge, and 256 request/restart interleavings.

The host must synchronously apply the command inside the callback and must not copy it into another queue without preserving/enforcing the epoch. An upstream sequence/replay guard remains responsible for lifetime request uniqueness; this queue rejects duplicate IDs only while the prior command remains pending.

## What remains outside the guarantee

This is not a production command queue and does not stop a second process or direct native-controller client from bypassing it. The callback API only serializes the same in-process owner; it cannot prove that a native actuator accepted the same token atomically with the actual write. It also cannot halt an action that has already crossed the hardware application boundary. Restart integration must still establish safe-state behavior, drain/cancel controller-owned work, and rely on separately qualified watchdog, collision monitor, and emergency-stop mechanisms.

For the production adapter, enforce the token again at the native command acceptance/write boundary; record both `command_epoch_at_application` and whether a stale command was actually accepted/applied. The latter is a separate non-compensatory hard gate from stale outcome-event acceptance and stale command admission. If the native boundary cannot report that evidence, the benchmark must mark the application-level result unobserved—not implicitly zero.

## Benchmark gates

- Stale epoch enqueue acceptance: 0.
- Expired command application acceptance: 0; a deadline miss that is explicitly rejected is not itself an application-boundary violation.
- Pending prior-generation commands executed after epoch advance: 0.
- Queue overflow must return an explicit rejection and must never silently overwrite an accepted command.
- Epoch authority unavailable/rolled back: no command application may be claimed authorized.
- Report queue depth, rejected/cancelled counts and admission-to-application latency by scenario and arm.
- Mock queue tests establish reference behavior only; simulator, hardware-in-the-loop, and physical results require separately identified evidence tiers.

## Industry context

ROS 2 Control separates controller lifecycle from the real-time update path and describes controller error/fallback behavior. Avoid blocking durable-store operations on every high-rate update; activate a monotonic generation outside the loop, then propagate and enforce it at the bounded native command application boundary. This reference queue is a testable contract for that boundary, not a replacement for `ros2_control`, hardware safety logic, or platform qualification.

References:
- ROS 2 Control controller manager: https://control.ros.org/kilted/doc/ros2_control/controller_manager/doc/userdoc.html
- RoboDojo evaluation protocol: https://github.com/RoboDojo-Benchmark/RoboDojo
- NIST response-robot performance measurement: https://www.nist.gov/programs-projects/performance-emergency-response-robots
