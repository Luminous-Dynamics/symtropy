# symtropy-robotics-bridge-core

Permissively-licensed (Apache-2.0 OR MIT) core types and traits for symtropy-robotics-bridge.

## Features

- PlatformType: enumeration of Symthaea robotics platforms.
- RoboticAgent: trait for agents that can be ticked by the game engine.
- ControlRequest / EmbodiedIntent: typed semantic requests with expiry, frame-aware targets and confidence validation.
- ControlOutcome: explicit lifecycle, constraint/veto feedback and request identity validation.
- RuntimeAssuranceGate: deterministic admission checks for request freshness, supported intents, explicit frames, workspace bounds, speed caps and independently observed health.
- RequestSequenceGuard: bounded per-session monotonic sequence tracking; duplicate and out-of-order sequence numbers are rejected without evicting tracked sessions.
- MotorPlanner: trait for per-joint motor command planning.
- spawn_robot_body: helper for spawning a platform-appropriate body into the physics world.

The control contract is intentionally platform-agnostic: it does not import symthaea-core or replace local controller/safety logic. It is a request/outcome seam for embodiment-specific adapters.

The runtime-assurance gate is an admission check, not a trajectory validator, collision checker, stability proof, certified safety function, autopilot replacement or permission for cognition to write actuators directly. A permit means only that the request may be handed to the platform adapter. Local controllers and independent safety authorities must continue enforcing limits during execution.

The sequence guard only protects monotonic ordering within the live session state supplied to it. It is not cryptographic authentication, does not prevent a sender from re-encoding an action under a fresh sequence, and is not durable across process restarts. Before deployment, integrate it atomically with the adapter dispatch path, authenticate and rotate session identifiers, and persist integrity-protected replay state where restart replay is in scope. Capacity exhaustion intentionally fails closed.

See [SYMPTHAEA_RUNTIME_ASSURANCE_GATE_V0_1.md](../../../docs/research/SYMPTHAEA_RUNTIME_ASSURANCE_GATE_V0_1.md) for the fault-injection matrix, proposed performance targets and benchmark baselines.

This crate contains NO AGPL-licensed dependencies and is safe for use in proprietary applications.
