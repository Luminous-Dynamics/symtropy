# Epoch-Fenced Differential-Drive Admission v0.1

**Status:** reference adapter and mock conformance tests authored; no native ROS 2 controller, IPC endpoint, simulator control loop, or physical hardware implementation is wired to this interface.

## Problem

The existing differential-drive adapter forwards session, sequence, request ID, and deadline in `CommandProvenance`, but no controller epoch reaches its native-controller trait. An outer in-process fence can reject an old token before calling that adapter, yet it cannot require an independently running native controller to validate the same token. This seam is where an implementation can accidentally lose generation identity.

## Included

- Adds `EpochFencedDifferentialDriveAdapter`, distinct from the untagged `DifferentialDriveAdapter`.
- The new adapter intentionally does not implement `ControllerSink`; every submission requires an explicit `ControllerEpoch`.
- Adds `EpochFencedCommandProvenance` with controller epoch, session, sequence, request ID, and monotonic expiry.
- Adds `EpochFencedDifferentialDriveLocalController::submit_goal_at_epoch`, which must be implemented by the hosting native adapter. Its contract requires atomic epoch and deadline checks at native command admission.
- Native receipts report the accepted generation and monotonic acceptance timestamp. Mismatch is surfaced as a hard-boundary violation; a report at or beyond the expiry is surfaced separately from an ordinary safe deadline rejection. The paired-recovery scorer must count both native epoch-mismatch acceptance and native expired-command acceptance as separate non-compensatory gates.
- Preserves semantic intent mapping, frame validation, speed bounds, and explicit errors for unsupported intents.
- Unit tests cover identity propagation, stale epochs at both adapter and native seams, expiry rejection, native epoch mismatch, late native acceptance, empty tickets, and monotonic advancement.

## Evidence boundary

These Rust types and mocks do not prove that a real native controller enforces the contract. The reported timestamp is native command admission, not proof a motor command was applied. ROS 2/IPC integration must atomically validate the epoch at its own admission point and report write/application evidence separately. If the native endpoint cannot supply that evidence, the corresponding benchmark fields are unobserved, not zero.

The host must obtain epochs from the persistent authority, coordinate activation with native controller lifecycle and queue draining, and retain independent collision monitoring, watchdog, actuator constraints, emergency-stop, and safe-state behavior. Do not put blocking durable-store queries in the high-frequency control loop.

## Conformance gates

- Stale adapter generation rejected before the native method: 0 native calls for that request.
- Native controller receives the same epoch, session, sequence, request ID and deadline that were admitted by the adapter.
- Native active epoch mismatch must fail closed.
- Native acceptance at or beyond the deadline is a non-compensatory application/acceptance failure, not a normal rejected deadline miss.
- A native epoch mismatch or expired acceptance blocks any task-performance win.
- Report native admission latency separately from actual hardware write/application latency.

## Industry context

ROS 2 Control already manages controller and hardware lifecycle, deactivation, and fallback after controller/hardware errors. This adapter is designed as a contract at the existing native boundary, not a replacement for the controller manager or platform safety system. Use the ROS 2 Control lifecycle and native command interfaces, then qualify any integrated implementation with simulator tests, HIL fault injection, and supervised physical tests.

References:
- ROS 2 Control controller manager: https://control.ros.org/kilted/doc/ros2_control/controller_manager/doc/userdoc.html
- NIST Performance of Emergency Response Robots: https://www.nist.gov/programs-projects/performance-emergency-response-robots
