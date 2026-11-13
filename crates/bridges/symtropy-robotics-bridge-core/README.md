# symtropy-robotics-bridge-core

Permissively licensed (Apache-2.0 OR MIT), dependency-light types and traits for the Symtropy robotics integration. The crate is kept separate from the AGPL Symthaea/Phi integration crate.

## What it provides

- PlatformType: a small platform taxonomy used by the bridge.
- RoboticAgent: a minimal observation/tick interface for research agents.
- MotorPlanner and UniformGainPlanner: command-planning interfaces and a scalar baseline.
- JointSafetyAuthority: a legacy per-joint command-attenuation helper with a local JointAuthorityTier type.
- spawn_robot_body: a coarse physics proxy helper.

## Important limits

These abstractions are not a production robot-control or safety stack.

- UniformGainPlanner applies a uniform gain; it is not a joint-space controller, impedance controller, whole-body controller or locomotion policy.
- spawn_robot_body creates a simplified physics proxy, not a CAD-derived articulated robot with qualified inertia, joints, sensors or actuator models.
- JointSafetyAuthority is only a scalar attenuation helper. It does not know actuator units or limits, hazard state, contact, gravity loading, safe-stop behavior or safety integrity requirements. Zero torque is not a universal safe state.
- Platform-specific, high-rate local controllers and an independent safety authority must own actuator limits, watchdogs, stale-command rejection, fault transitions and the defined physical safe state.
- The full Symthaea/Bevy/consciousness-physics integration lives in the sibling AGPL symtropy-robotics-bridge crate.

This crate does not depend on symtropy-consciousness-physics or Symthaea. The local attenuation enum is intentionally not the AGPL research layer's Phi-derived SafetyTier. Check the exact dependency graph of the release you consume; a package's license field alone is not a substitute for dependency review.

## Related work

- [Control authority boundary #2828](https://github.com/Luminous-Dynamics/symtropy/issues/2828)
- [Typed embodiment contract PR #2829](https://github.com/Luminous-Dynamics/symtropy/pull/2829)
- [Fail-closed bridge validation PR #2167](https://github.com/Luminous-Dynamics/symtropy/pull/2167)
