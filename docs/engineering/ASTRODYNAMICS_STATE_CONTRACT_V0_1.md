# Astrodynamics state and frame contract — v0.1

**Status:** proposed engineering contract; not a validated dynamics implementation.  
**Owner:** Symtropy geometry / simulation substrate.  
**Consumers:** Symthaea orbital-ring research and any space simulation that needs explicit frame semantics.

## Purpose and scope

Define the minimum semantics required before a space model may exchange state between Symtropy geometry, Symthaea dynamics, a research evidence pipeline, or a renderer.

This contract does **not** claim orbital-ring feasibility. Symtropy's existing spatial Transform and Rotor abstractions provide geometric operations; by themselves they are not complete time-dependent astrodynamics frames. The existing spacecraft orbital demo is not an orbital-ring solver.

## Separate five claims that are often conflated

1. **Representation validity:** inputs, units, frame identifiers, epochs and rotations are well formed.
2. **Kinematic consistency:** positions, velocities and frame transforms agree under the stated convention.
3. **Dynamic equilibrium:** all modeled force terms balance within justified tolerances for one specified architecture.
4. **Local stability:** the defined equilibrium remains bounded under a declared perturbation model; report eigenmodes or other appropriate stability evidence.
5. **Fault survivability:** a specified disturbance stays inside verified hardware, geometry, energy and momentum bounds.

Passing an earlier level never promotes a later level automatically. In particular, a small conservation residual does not prove equilibrium, stability, or survivability.

## Canonical state envelope

Every cross-crate state exchanged for physical analysis must identify:

- **State identity:** schema/version, deterministic case ID, parent/source manifest digest and immutable artifact references.
- **Frame identity:** explicit frame name/realization, origin, axes, handedness, epoch and time scale. An unqualified label such as “ECI” is not sufficient for frame-critical calculations.
- **Position:** finite 3-vector in metres and an explicitly stated origin.
- **Velocity:** finite 3-vector in metres per second, with its reference point, epoch and frame. Record whether it is inertial or frame-relative.
- **Attitude:** explicit convention and representation; validate normalization or rotation-matrix orthogonality and determinant before accepting it.
- **Angular state:** angular velocity and, when used, angular acceleration, including the frame in which each is expressed.
- **Model assumptions:** gravity model, force models enabled/omitted, mass representation, drag treatment and valid domain.
- **Uncertainty:** source uncertainty, numerical tolerances, and the evidence justifying each acceptance threshold.

Use SI internally. Reject NaN, infinities, impossible dimensions, unlabelled frame changes, and missing epochs at the boundary rather than inventing defaults.

## Transform semantics

A spatial transform of the form p' = R p + t may transform a point; a pure rotation may transform a geometric vector. Neither operation alone supplies the time derivatives needed for a moving or rotating reference frame.

Let R map components expressed in frame B into inertial frame I; let r_B be a point relative to B's origin, and express the frame's angular velocity and angular acceleration in I. Then the transform must account for:

- Position: r_I = r_O,I + R r_B.
- Velocity: v_I = v_O,I + R v_B + omega_I/B × (R r_B).
- Acceleration: a_I = a_O,I + R a_B + 2 omega × (R v_B) + alpha × (R r_B) + omega × (omega × (R r_B)).

These identities state the convention this interface must implement or explicitly delegate. They are not a force law and do not determine orbital-ring support forces. Tests must use the same declared frame and sign convention.

Every transform must state its direction (from-frame and to-frame), epoch, origin state, orientation convention, and angular kinematics when derivatives are required. Composition must not silently reverse the order of transforms.

## Reference basis and independent comparison

The transform conventions above should be cross-checked against mature, independently maintained astrodynamics implementations before this API supports engineering claims:

- [IAU SOFA](https://www.iausofa.org/) provides IAU-endorsed algorithms for astronomical reference systems, time scales and related transformations. Use the applicable routines and published conventions where the chosen frame requires them.
- [Orekit frame architecture](https://www.orekit.org/site-orekit-9.3/architecture/frames.html) documents time-dependent frame trees and transforms with the derivatives needed for velocity conversion. Use it as a candidate independent reference for transform/kinematic fixtures, not as a substitute for specifying the frame realization and epoch.
- [NASA GMAT verification and validation](https://ntrs.nasa.gov/citations/20140017798) is a useful precedent for regression against independent reference data. Re-running the same implementation twice establishes determinism, not independent correctness.

These are references for the mathematics and verification process only. None of them validates the continuous-skyhook architecture, its support loads, a ring equilibrium, or its stability.

## Physics authority boundary

- Orbital forces and state propagation must derive from declared physical models and units.
- Phi, trust, reputation, policy scores, or other cognitive/social metrics are not physical forces. They must not alter gravitational parameters, conservation equations, stability thresholds, or engineering acceptance limits unless a separate, physically justified and independently validated constitutive model explicitly defines that coupling.
- A controller or agent may propose an action; the physics model and safety gate independently evaluate it.
- The renderer may visualize a result but must never upgrade its evidence state.

## Required verification matrix for a future implementation

| Check | Minimum acceptance evidence |
|---|---|
| Frame metadata | Missing frame, epoch or transform direction is rejected |
| Rotation | Finite values, orthogonality error and determinant +1 within justified tolerances |
| Transform algebra | Identity, inverse round trip and composition tests |
| Relative kinematics | Static, translating and rotating-frame fixtures with independently hand-checkable values |
| Circular two-body reference | Compare radius, speed, period and specific orbital energy against closed-form equations |
| Sensitivity | Show tolerance/time-step convergence; retain failed or divergent cases |
| Physical equilibrium | A separately specified architecture, complete force inventory and force-balance residual |
| Linear stability | Equilibrium-specific perturbation/eigenmode method with model and boundary assumptions stated |
| Fault response | Disturbance definition, actuator/coupler limits, energy and momentum boundaries, and an independent reference case |
| Provenance | Source and output hashes, exact command/runtime, deterministic repeat output, explicit claim ceiling |

A basic circular-orbit reference only validates the free two-body limiting case. It does not validate a stationary ring, skyhook, launch loop, electromagnetic support, structural dynamics, thermal budget, debris response, or construction feasibility.

## Implementation order

1. Define shared frame and state types with typed SI quantities and explicit frame/epoch metadata.
2. Add fail-closed transform and rotating-frame kinematics fixtures.
3. Cross-check the free-orbit reference with an independent implementation/tool and a frozen input manifest.
4. Specify one real architecture and complete its force and conservation boundaries before implementing equilibrium.
5. Implement equilibrium and stability as distinct, separately evidenced stages.
6. Connect immutable run receipts and read-only visualizations only after the state/evidence schemas are stable.

Until these prerequisites are satisfied, downstream consumers must treat unsupported frame semantics or missing dynamics as **Blocked**, not as zero-valued or successful state.
