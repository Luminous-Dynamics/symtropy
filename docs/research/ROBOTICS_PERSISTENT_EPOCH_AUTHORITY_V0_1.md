# Persistent Controller Epoch Authority Contract v0.1

**Status:** Rust reference contract and adversarial mock tests; no production persistent backend is implemented.
**Parent:** #2868 (in-memory epoch fence).
**Primary question:** after restart, store outage, failover, or rollback, can the system prove which controller generation is current without inventing or resurrecting a token?

## Paradigm shift: a generation is a capability, not a counter

An in-memory counter is useful for unit tests but does not establish controller identity across process lifetime. A process ID, wall-clock timestamp, boot-time estimate, randomly generated identifier, restored snapshot, or caller-supplied integer is not by itself proof that a generation is authoritative and current.

The authority must supply a monotonic fencing generation tied to a stable controller identity. The store contract has two operations:
- `read_generation(controller_id)`: return the existing authoritative generation or a hard error. A missing record must not silently initialize to generation 1.
- `compare_and_advance(controller_id, expected)`: atomically compare the durable generation and durably commit exactly `expected + 1`, or reject a stale contender. The value must be committed before a successful response is returned.

The Rust trait cannot prove a concrete storage backend honors durability, linearizability, monotonicity through rollback, or failover guarantees. Those properties require backend-specific fault injection and operational qualification.

## Implementation in this PR

`epoch_authority.rs` adds:
- `ControllerEpochStore`: an explicit storage contract with read and atomic compare-and-advance operations.
- `ControllerEpochAuthority`: binds a stable controller ID to the storage interface.
- `claim_next_epoch()`: reads the current generation, requests an exact one-step advance, verifies the returned value, updates a retained high-water mark, and reads back to detect immediate supersession.
- `ensure_current(epoch)`: rejects a stale epoch and detects generation rollback relative to the wrapper instance's highest observed generation.
- Structured failures for store unavailability, missing/corrupt state, compare-and-advance conflict, unexpected generation increments, rollback, overflow, supersession, and stale tokens.
- No local fallback counter or “initialize at one” recovery path.
- A reference mock store and tests for unavailable/corrupt/missing state, wrong store response, rollback, supersession, overflow, stale tokens, and competing authority clients sharing one atomic store.

## Important limits

The wrapper's high-water mark is process-local. It detects rollback while that authority instance retains its memory, but it cannot detect a storage rollback after *all* clients and high-water marks have been lost. Durable rollback resistance must live in the persistent authority, e.g. an independent monotonic/consensus-backed store, hardware-backed monotonic primitive where appropriate, or a separately protected authoritative checkpoint.

`ensure_current` is a point-in-time read. It does not eliminate the time-of-check/time-of-use race between verifying a token and applying a physical command. Native command acceptance must enforce the token as part of its atomic admission operation, or all command acceptance must be serialized behind one authoritative owner. The epoch layer is not an e-stop and does not stop a previously accepted command.

A backend must also make its compare-and-advance operation linearizable across independent processes. A mutex around a single process, SQLite-like local state without a rollback threat model, or an in-memory mock is not evidence of cross-host fencing.

## Relation to robotics controller lifecycle

ROS 2 Control's controller manager exposes controller lifecycle management and documents restarting controllers, hardware lifecycle reconfiguration, and error-driven controller deactivation/fallback. Its real-time control loop has timing constraints, so a persistent epoch-store query should not simply be inserted into every high-frequency update callback. Instead, use the authority to establish an incarnation token, propagate that token to the native control boundary, and ensure the boundary enforces the token on command admission. See:
- https://control.ros.org/kilted/doc/ros2_control/controller_manager/doc/userdoc.html
- https://control.ros.org/master/doc/ros2_control/controller_manager/doc/userdoc.html
- https://control.ros.org/master/doc/ros2_control/doc/getting_started/getting_started.html

This PR is not a ROS 2 plugin, does not implement a storage service, and is not integrated with any real controller manager. It defines the seam and its failure semantics first.

## Benchmark gates

For a candidate production backend, the following should be mandatory:

1. Cold start with a valid existing generation uses that generation; missing, corrupt, or unavailable state never invents a new generation.
2. Two or more concurrent claimers cannot both end the activation step with a token that the authority considers current.
3. A stale contender receives a conflict or is detected as superseded; it must not continue as current.
4. After a restart, the new generation is strictly greater than every generation previously accepted by the authoritative system.
5. Snapshot rollback, failover to a stale replica, authority-store outage, and partial commit are tested explicitly.
6. A command carrying a stale token is rejected *atomically at the command boundary*, not merely by a previous read.
7. Report claim latency and p50/p95/p99/max, availability, CAS conflicts, supersession, failed closed activations, stale-token rejections, false accepts, and missing telemetry. Failed and timed-out attempts remain in the denominator.
8. Distinguish tests against a mock, a single-host persistent backend, and a real distributed/independent authority. They prove different properties.

**Proposed hard gate:** zero stale-token or fail-open command admissions in the frozen negative campaign. This is a test criterion, not a claim that zero real-world risk has been proven.

## Evidence claim ceiling

The new tests run against a mock store whose operations use an in-process mutex. They test the Rust wrapper's error handling, but they do not establish durable commit, host restart survival, linearizability across hosts, anti-rollback storage, or physical-command enforcement. Do not claim durable anti-replay or production fencing until a real backend and native command-boundary integration pass the above qualification.
