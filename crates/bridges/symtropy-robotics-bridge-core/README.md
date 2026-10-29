# symtropy-robotics-bridge-core

Permissively-licensed (Apache-2.0 OR MIT) core types and traits for symtropy-robotics-bridge.

## Features

- PlatformType: enumeration of Symthaea robotics platforms.
- RoboticAgent: trait for agents that can be ticked by the game engine.
- ControlRequest / EmbodiedIntent: typed semantic requests with expiry, frame-aware targets and confidence validation.
- ControlOutcome: explicit lifecycle, constraint/veto feedback and request identity validation.
- RuntimeAssuranceGate: deterministic admission checks for request freshness, supported intents, explicit frames, workspace bounds, speed caps and independently observed health.
- RequestSequenceGuard and SequencedRuntimeAssurance: bounded per-session monotonic sequence tracking and pre-admission duplicate/order rejection.
- DispatchBoundary: serializes authentication, sequence/gate admission and synchronous controller submission within one mutable authority instance.
- MotorPlanner: trait for per-joint motor command planning.
- spawn_robot_body: helper for spawning a platform-appropriate body into the physics world.

The control contract is intentionally platform-agnostic: it does not import symthaea-core or replace local controller/safety logic. It is a request/outcome seam for embodiment-specific adapters.

The runtime-assurance gate is an admission check, not a trajectory validator, collision checker, stability proof, certified safety function, autopilot replacement or permission for cognition to write actuators directly. Local controllers and independent safety authorities must continue enforcing limits during execution.

DispatchBoundary requires a configured SessionAuthenticator and ControllerSink. The boundary owns these components and sequence state; callers must serialize access to the same instance, or hold one mutex across the entire dispatch call. Multiple independently instantiated boundaries do not share anti-replay state. The authenticator is a trust seam: it must actually validate credentials, revocation and non-reused session identity. AuthenticatedSession is not itself a cryptographic proof.

The sink receipt only means the adapter accepted a submission ticket. It does not prove motion occurred or that the task completed. The boundary is currently tested against mock sinks; it is not yet wired to the real multirotor or non-drone paths. Persistent replay protection across restarts, multi-process atomicity, trajectory collision checking and certified safety are out of scope.

See [SYMPTHAEA_RUNTIME_ASSURANCE_GATE_V0_1.md](../../../docs/research/SYMPTHAEA_RUNTIME_ASSURANCE_GATE_V0_1.md) for the fault-injection matrix, proposed performance targets and benchmark baselines.

This crate contains NO AGPL-licensed dependencies and is safe for use in proprietary applications.
