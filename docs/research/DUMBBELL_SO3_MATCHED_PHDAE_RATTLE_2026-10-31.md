# Matched 3D SO(3) pHDAE discrete-gradient vs Cartesian RATTLE dumbbell

**Status:** experimental matched numerical comparison with an exact Rodrigues trajectory oracle; no external-engine measurements.  
**Date:** 2026-10-31  
**3D pHDAE reference:** [#3134](https://github.com/Luminous-Dynamics/symtropy/pull/3134).  
**Research objective:** [#3132 PHYS-PHDAE-SO3-001](https://github.com/Luminous-Dynamics/symtropy/issues/3132).  
**Qualification owner:** [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Purpose

This campaign adds an independent Cartesian 3D RATTLE baseline to the 3D semi-explicit pHDAE discrete-gradient dumbbell. Both methods receive exactly the same state, masses, link length, time horizon and timestep ladder. The common trajectory reference is exact free rigid-link motion with Rodrigues rotation of the relative state and uniform center-of-mass translation.

The campaign is deliberately multi-observable. It does not reduce structure preservation to energy drift alone.

## Frozen 3D problem

- m1=1.3 kg, m2=2.1 kg, L=1.2 m.
- Center position R=(0.25,-0.12,0.30) m.
- Center velocity V=(0.17,-0.23,0.11) m/s.
- Relative position r=(0.8,0.8,0.4) m.
- Relative velocity w=(0.2,-0.1,-0.2) m/s.
- r·w=0; all components of r, w and angular velocity are nonzero.
- No external force or torque for the matched trajectory case.
- Final time T=1 s.
- Timesteps h=0.04, 0.02, 0.01, 0.005 s.
- Four step sizes × two methods = eight matched output rows.

The exact reference is

\[
R_{cm}(t)=R_{cm}(0)+V_{cm}(0)t,
\]

\[
\omega=\frac{r_0\times w_0}{L^2},\quad
r(t)=\operatorname{Rodrigues}(\omega t)r_0,\quad
w(t)=\operatorname{Rodrigues}(\omega t)w_0.
\]

Each body state is reconstructed from the mass-weighted center-of-mass and relative coordinates. This oracle does not call either solver's step or multiplier code.

## Methods

### Semi-explicit pHDAE discrete gradient

The midpoint relative link vector is the exact discrete Jacobian of the squared-distance constraint. A scalar multiplier enforces the midpoint algebraic velocity constraint. This preserves the link position and makes the constraint reaction do zero discrete work. The full angular momentum vector is recorded.

### Cartesian RATTLE

The independent RATTLE implementation solves the quadratic position multiplier to enforce the distance constraint, then computes a separate endpoint velocity multiplier to enforce tangency at the new position. It is symplectic for this constrained mechanical system; energy error is recorded rather than post-corrected.

## Report schema

The deterministic CSV contains:
- final state error against the Rodrigues solution;
- maximum relative energy drift;
- maximum relative link constraint;
- maximum relative midpoint velocity-constraint diagnostic;
- maximum absolute endpoint tangent residual;
- maximum relative linear momentum residual;
- maximum relative full angular momentum-vector residual;
- mean and maximum multiplier iterations.

The receipt auditor rejects unknown methods, wrong timestep/step count, duplicate or missing method/timestep rows, malformed/non-finite/negative numeric fields, excessive residuals and impossible iteration accounting. The test suite injects NaN, duplicate rows and a missing row. This is a structural/report invariant audit, not an independent reimplementation of either integrator.

## Independent equation-level numerical cross-checks

These figures come from separate double-precision implementations of the equations and are **not** Rust CI output.

### Trajectory convergence

| h (s) | pHDAE final state error | RATTLE final state error | RATTLE / pHDAE |
|---:|---:|---:|---:|
| 0.040 | 1.8719e-6 | 3.7440e-6 | 2.0001 |
| 0.020 | 4.6798e-7 | 9.3598e-7 | 2.0000 |
| 0.010 | 1.1700e-7 | 2.3399e-7 | 2.0000 |
| 0.005 | 2.9249e-8 | 5.8498e-8 | 2.0000 |

Both methods converge at second order. In this particular smooth free-dumbbell case, pHDAE has about half the terminal state error of RATTLE at the same step size. This is a fixture-specific error-constant result, not a general ranking.

### Structure residuals

At h=0.002 over 5,000 steps, the independent equation-level checks found:

| Observable | pHDAE | RATTLE |
|---|---:|---:|
| Link residual | ~3e-15 | ~2e-16 |
| Midpoint algebraic constraint | ~3e-15 | ~4e-14 |
| Endpoint tangent residual | ~4e-15 | ~6e-17 |
| Relative energy drift | ~1e-15 | ~1e-12 |
| Relative linear momentum drift | ~3e-15 | ~7e-13 |
| Relative angular momentum vector drift | ~7e-15 | ~5e-13 |

The exact values depend on accumulated floating-point rounding and should be replaced by the Rust CSV once the exact-head workflow passes. The important point is that the two methods' defining properties remain separately visible: pHDAE power/energy structure and RATTLE endpoint-constraint structure. The special torque-free dumbbell happens to make endpoint tangency roundoff-small for the pHDAE scheme as well; do not generalize this exceptional case to arbitrary semi-explicit pHDAEs.

## Industry comparison and next frontier

This is not yet a MuJoCo, Rapier, Chrono or PhysX comparison. The canonical external differential runner remains [#443](https://github.com/Luminous-Dynamics/symtropy/issues/443), with Rapier version and determinism profile pinned as separate evidence identities.

The next external comparison should use a genuine 3D distance-joint/constraint in each engine and freeze:
- exact engine version and backend profile;
- integration scheme, solver iteration count, warm-starting and stabilization;
- masses, link geometry, initial state, units, gravity and time step;
- output cadence, thread count, build identity and hardware.

Measure trajectory error, energy drift, position/endpoint/midpoint constraints, full angular-momentum vector, linear momentum, failure rate, p50/p95/p99 step time, memory and verified simulated time per CPU-second. Do not compare a 2D hinge to a 3D distance constraint or silently substitute a fixed joint for a compliant joint.

## Claim ceiling

This is a spatial two-point-mass, fixed-link, force-free numerical comparison. It does not implement general rigid-body orientation dynamics, articulated chains, contact, impact, friction, adaptive stepping, sparse DAE infrastructure or PhysicsWorld integration. No SOTA, production, or physical-validation claim is made.

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/phdae-dumbbell-3d-reference/constrained_dumbbell_3d.rs
rustup run 1.96.0 rustfmt --edition 2021 --check tools/dumbbell-rattle-3d-reference/cartesian_rattle_3d.rs
rustup run 1.96.0 rustfmt --edition 2021 --check tools/dumbbell-matched-3d-campaign/compare_dumbbell_3d_solvers.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/dumbbell-matched-3d-campaign/compare_dumbbell_3d_solvers.rs -o /tmp/dumbbell-so3-tests
/tmp/dumbbell-so3-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/dumbbell-matched-3d-campaign/compare_dumbbell_3d_solvers.rs -o /tmp/dumbbell-so3
/tmp/dumbbell-so3 > /tmp/dumbbell-so3-a.csv
/tmp/dumbbell-so3 > /tmp/dumbbell-so3-b.csv
cmp /tmp/dumbbell-so3-a.csv /tmp/dumbbell-so3-b.csv
~~~
