# Careframe Digital-Twin / Morphology Contract v0

**Status:** synthetic graph and geometry-envelope contract only. This is not an imported URDF, a validated multibody model, manufacturing CAD, a physical robot specification, a risk-management file, or authority to build/power a device.

## Why this layer is necessary

Symtropy's existing generic robotics bridge currently uses a single bounding sphere when spawning a platform body. Its generic joint metadata contains a name and motor index; the bridge's generic motor path does not, by itself, provide a complete humanoid kinematic chain with per-joint axis, limits, link geometry, tool frame and measured actuator dynamics. Those interfaces remain useful for coarse prototypes but cannot be treated as a validated articulated humanoid model.

Careframe therefore begins with a distinct named, versioned morphology contract. This is a proposal interchange file and graph check; it does not claim the current engine imports it as a full articulated robot. The next adapter must translate the representation into the actual Symtropy joint/collider/transform implementation and prove parity with the manifest in tests.

## Model fixture

`careframe-wheeled-upper-body-morphology-v0.json` describes a **wheeled upper-body humanoid concept** with 16 links and 15 joints:
- wheeled base with two drive-wheel placeholders and two caster placeholders;
- torso yaw carriage and torso shell;
- sensor mast and two-axis head assembly;
- two arm chains with upper-arm, forearm and tool-mount links.

All link geometry uses conservative bounding boxes. Every size, mass, joint origin, effort limit and velocity limit is an invented synthetic placeholder used to test the graph/validator—not measured, optimized, or suitable as an engineering specification. Even the nominal aggregate mass is only arithmetic over those placeholders, not a weight estimate for a build.

## Strict invariants

- Exact schema version and morphology ID/revision.
- Every link and joint has a unique non-empty ID.
- Exactly one declared root link; root has no incoming joint; every other link has exactly one parent joint.
- Each joint points to known, distinct parent and child links; the whole graph is connected and acyclic.
- Positive finite link masses and collision dimensions; bounded dimensions; collision shape and geometry-source values are allowlisted.
- Revolute joint axis is a finite, non-zero unit vector; lower limit is less than upper; effort and velocity envelopes are positive finite. Fixed joints require zero axis, zero joint limits and zero actuator envelope.
- Every joint's command-enabled flag is false. Controller command outputs are disabled. These assertions make this file a kinematic graph fixture; it is not a motor-control input.
- The model is synthetic-only; physical actuation, patient contact and clinical use authority are always false.
- Unknown fields reject rather than being silently ignored.
- Validator receipt binds to the exact model file digest, validator source digest and tested source revision.

## Forward-kinematics evaluator

The companion [Forward-Kinematics Contract](CAREFRAME_FORWARD_KINEMATICS_CONTRACT_V0.md) now specifies a deterministic, geometry-only evaluation path. The Rust tool accepts a morphology file and an exact state file, rejects missing/unknown/out-of-range revolute-joint state, computes a pose for every link, and emits a receipt bound to the input digests and source revision. The zero-state reference points and a nonzero torso-yaw transform are covered by independent expected-coordinate assertions.

This is **not yet an articulated Symtropy/Bevy physics adapter**. It validates one declared transform convention against synthetic reference points; it does not check collider installation, engine transforms, dynamics, contacts, or real hardware.

## URDF interchange output

A separate Rust exporter now emits URDF 1.0 kinematic/collision data from the validated synthetic model. It includes link bounding boxes and the current fixed/revolute joint graph, but intentionally emits no inertial data, transmissions or controller interfaces. The generated XML is parsed in CI and its 16-link/15-joint structure is checked. This is a bridge to independent kinematic tooling—not proof of compatibility with a running ROS graph, a MuJoCo dynamics model, or the Symtropy/Bevy engine.

## What the contract does not cover

The current fixture does not contain validated inertias, centre-of-mass measurements, joint friction/backlash, gearbox or tendon models, cable routing, reach maps, self-collision simplification error, thermal/electrical behavior, emergency-stop response, human factors, payload-induced stability, actuator control, cleanability, material compatibility or clinical efficacy. Uniform-density primitive dynamics (if derived later) must be marked an approximation and compared against an independently constructed reference model.

The fixture's joint effort and speed entries are **not** safe contact limits, approved operating limits or certified actuator selections. They must not be sent to real hardware.

## Conformance benchmark

- Accept the frozen model fixture.
- Reject unknown top-level, link, collision, joint, controller and authority fields.
- Reject duplicate IDs; missing/duplicate roots; unknown links; self edges; multiple parents; disconnected components and cycles.
- Reject non-finite/out-of-range geometry and mass, zero/unnormalized revolute axes, inverted/degenerate revolute ranges and impossible fixed-joint envelopes.
- Reject any joint command enabled, any controller outputs enabled, and any physical/patient/clinical authority true.
- Mutate each graph and authority invariant independently. Every rejection receipt must include a deterministic finding code and exact model digest.

## Integration gates

1. **Graph contract:** this Rust validator establishes only JSON/schema/topology eligibility.
2. **Geometry adapter:** convert to engine-side joints/colliders and compare calculated transforms/bounds against independent fixtures; no motor commands.
3. **Kinematic checks:** reachable workspace, self-collision approximations, joint-limit enforcement and tool-flange frame tests in synthetic scenes.
4. **Dynamics characterization:** only after explicit model parameters and independent reference comparisons; never infer physical inertias from this proposal.
5. **Non-clinical guarded bench/HIL:** only after separately approved risk/equipment plan.
6. **Any human or patient-facing study:** new intended-purpose, risk, ethics, cybersecurity, clinical and regulatory gates; not authorized by this contract.

## Evidence ceiling

A passing validator may state only: **"the synthetic articulated-morphology JSON satisfied the declared graph and envelope contract at the recorded revision."** It cannot state that the robot is physically buildable, dynamically accurate, safe, medically suitable, or better than any industry system.
