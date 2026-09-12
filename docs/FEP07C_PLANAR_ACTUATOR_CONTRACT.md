# FEP-07C — bounded planar locomotion actuator v0.1

## Status

Implemented source contract only. This document is not runtime qualification.

The tranche is stacked on the FEP-07 fixed-physics prerequisite and introduces a launcher-local motor boundary before player/NPC movement is rewired to use it.

## Problem

The 2D launcher currently has three different concepts that must not be conflated:

1. **intent** — player input or an NPC `MoveTarget`;
2. **self-propelled motor authority** — what an entity is allowed/able to command;
3. **physical motion** — authoritative `PhysicsWorld` velocity, collision response and position.

The pre-FEP-07 NPC adapter collapsed all three into a direct `RigidBody::linear_velocity = ...` write. That bypasses acceleration limits and makes locomotion energy accounting ambiguous.

## v0.1 authority rule

`apply_planar_motor_target` is the only primitive introduced by this tranche.

It accepts:

- one authoritative `BodyHandle`;
- a desired planar velocity;
- explicit maximum speed and acceleration;
- one fixed timestep.

It then:

1. rejects non-finite/invalid requests;
2. requires a dynamic finite positive-mass body;
3. requires registered consciousness/energy motor state;
4. validates the reservoir before trusting cached motor authority;
5. returns zero self-propelled authority for collapsed/effectively empty reservoirs without damping existing momentum;
6. computes the currently extractable mechanical-work budget;
7. clamps requested speed;
8. scales both target speed and acceleration authority by the entity's current effective motor gain;
9. limits the per-step velocity correction by acceleration;
10. limits positive motor work to the extractable-work budget before motion is applied;
11. separates positive work from negative-work braking even when a reversal crosses zero velocity inside one step;
12. debits positive motor work exactly once;
13. dissipates braking work rather than granting regenerative credit;
14. applies only the bounded velocity correction to the authoritative body.

This is a velocity-target **actuator**, not a direct velocity setter: the target may take many fixed ticks to reach.

## Why v0.1 uses bounded delta-v rather than a new core force ABI

A generic force-channel redesign would change the semantics of the ND physics core and every callback consumer. That is larger than necessary to prove the launcher contract.

The bounded delta-v controller is mechanically interpretable as a center-of-mass impulse spread over one fixed step:

`impulse = mass × delta_velocity`

The acceleration bound limits that impulse. Mechanical work is derived on the same correction axis, so self-propulsion can be accounted independently of collision impulses and unrelated external motion.

If the contract proves useful under execution, it can later be generalized into a reusable core actuator API without changing the observable launcher theorem.

## Positive work versus braking

Net kinetic-energy change is **not** sufficient for accounting.

A command can reverse a body from `-v` to `+v` and have nearly zero net kinetic-energy change while still containing two physically distinct phases:

- negative work while braking to zero;
- positive work while accelerating away from zero.

v0.1 decomposes those phases along the actuator correction axis.

Positive work consumes entity energy. Negative work is treated as dissipative braking and becomes heat/dissipation. Regenerative locomotion is intentionally **not** enabled in this tranche.

This avoids a reversal becoming a free acceleration because braking and propulsion cancelled numerically.

## Extractable-work theorem

Raw internal energy is not automatically equivalent to mechanically extractable work.

Under `consciousness-runtime`, the owning `EnergyBudget` already defines Helmholtz free energy through:

`available_work = max(U - T*S, 0)`

and documents that quantity as the maximum extractable work at constant temperature.

The actuator therefore caps **positive motor work by `available_work()`**, not by raw `energy.available`.

A body can consequently retain positive internal energy while having less—or zero—currently extractable mechanical work because of its temperature/entropy state.

The standalone/default launcher stub does not model temperature or entropy. Its explicit v0 fallback treats finite raw reservoir energy as the work budget. That is a simplified simulation fallback, not a claim that it implements the full Helmholtz model.

Braking-to-zero may still proceed without positive work. Acceleration beyond zero is work-limited separately.

## Reservoir-before-cache theorem

`EntityConsciousness::effective_motor_gain()` is cached safety-tier gain multiplied by motor precision. It does not itself read the energy reservoir at call time.

Therefore the actuator validates reservoir state **before** it trusts that cached gain.

A collapsed or effectively empty reservoir returns no self-propelled authority even if a stale cached tier still says Green or Yellow.

Invalid/non-finite reservoir or thermal state fails closed as an invalid motor state rather than mutating physical velocity.

This is a defense-in-depth authority boundary. The schedule should still update collapse/tier state correctly; the actuator must not depend on that timing for basic safety.

## Zero authority theorem

A zero effective motor gain or exhausted reservoir performs no velocity mutation at all.

In particular, the actuator does **not** set velocity to zero when an entity is Red/collapsed. This preserves the distinction between:

- no self-propelled motor authority; and
- no physical momentum.

A motor-disabled body may therefore continue moving because of a collision or another external cause.

## Existing motor-gain semantics are inherited, not validated here

Under `consciousness-runtime`, v0.1 consumes the existing `EntityConsciousness::effective_motor_gain()` result only after the direct reservoir gate succeeds. Standalone builds mirror the current stub tier/precision semantics.

The consciousness-physics `SafetyTier` module itself explicitly documents that its hard-coded Phi cut points are not calibrated to the measured bands of the robotics platform demos. This tranche does **not** upgrade those thresholds into a real-world safety claim or certification.

FEP-07C proves only that launcher locomotion cannot bypass the owning domain's current motor-authority value or the reservoir/work boundary.

A later calibration tranche should decide whether this game slice continues using the four-tier mapping, uses a slice-specific calibrated mapping, or consumes a different bounded control-authority provider.

## No double movement charge

`ThermodynamicConstants` already contains `movement_cost_per_unit` and `sprint_cost_multiplier`, but the inspected launcher thermodynamic pass does not presently debit them.

v0.1 does **not** begin charging those abstract per-distance constants in addition to mechanical motor work. Doing both without an explicit decomposition would double-charge locomotion.

For this tranche:

- positive actuator mechanical work is the locomotion debit;
- braking mechanical energy is dissipation;
- existing maintenance, collision, regeneration and other domain costs remain separate;
- the older per-distance movement constants are left untouched pending an explicit reconciliation/deprecation decision.

If a later design wants metabolic cost beyond mechanical work, it must be named separately (for example basal locomotor inefficiency) and tested as an additional model term rather than silently reusing the old movement constant.

## Player and NPC integration target

This tranche deliberately does **not** yet choose gameplay tuning or rewrite the large FEP system.

The next adapters should converge on the same primitive:

`player keyboard sample (Update)`
→ `PlayerInput`
→ fixed player intent adapter
→ `apply_planar_motor_target`

and

`FEP perception/reasoning`
→ `MoveTarget`
→ fixed NPC intent adapter
→ `apply_planar_motor_target`

Only the intent-to-desired-velocity policy should differ. The physical authority boundary should not.

## 3D boundary

Old Waterworks `Playing3D` remains kinematic and out of scope. This actuator must not silently become a second 3D movement authority.

## Source-level regression theorems

The implementation includes tests intended to establish, once executed:

- non-finite commands fail without mutating velocity;
- speed/acceleration requests are bounded;
- positive motor work is charged;
- extractable work limits acceleration before motion is applied;
- braking dissipates kinetic energy without regenerative credit;
- reversal cannot cancel paid acceleration against prior braking;
- Red motor authority does not erase externally induced velocity;
- a zero reservoir defeats stale cached Green authority without erasing external momentum;
- a collapsed full-runtime reservoir defeats contradictory cached Green authority;
- non-finite thermal state fails closed;
- reduced motor precision reduces motor authority.

## Required qualification

Before merge readiness, execute on the exact PR head:

- `cargo fmt --check`;
- default launcher check/test;
- launcher check/test with `--features consciousness-runtime`;
- launcher check/test with `--features fep-ai`;
- Clippy on the same relevant feature surfaces;
- a fixed-step scenario showing actuator request → authoritative body velocity → world integration → transform export;
- a low-energy scenario proving movement is capped before energy exhaustion;
- a high-entropy/low-free-energy scenario proving positive work is limited by Helmholtz `available_work()` rather than raw internal energy;
- a stale cached-tier scenario proving zero/collapsed reservoir state blocks self-propulsion immediately;
- a collision/external-velocity scenario proving zero motor authority does not erase momentum;
- a two-render-cadence replay with frozen seed/input tape/fixed-step count.

No exact-head execution PASS is claimed by this document.
