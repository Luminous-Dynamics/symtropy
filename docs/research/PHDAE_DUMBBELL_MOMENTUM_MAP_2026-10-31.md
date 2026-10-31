# G-equivariant semi-explicit pHDAE discrete-gradient constrained dumbbell

**Status:** experimental 2D symmetry / momentum-map reference; no PhysicsWorld integration.  
**Date:** 2026-10-31  
**Research parent:** [#3119 PHYS-PHDAE-SYMMETRY-001](https://github.com/Luminous-Dynamics/symtropy/issues/3119).  
**Shared qualification owner:** [#460 PHYS-VALID-06](https://github.com/Luminous-Dynamics/symtropy/issues/460).  
**Method context:** [#3118 semi-explicit pHDAE pendulum](https://github.com/Luminous-Dynamics/symtropy/pull/3118).

## Why this is the first momentum-map case

The gravitational pendulum is useful for energy and holonomic constraints, but gravity introduces an external preferred direction. It cannot test conservation of the full planar angular-momentum map.

The two-mass dumbbell is a minimal translation- and rotation-invariant multibody DAE:
- two positive point masses;
- one fixed-length, massless link;
- no external force or torque for the isolated benchmark;
- nonzero center-of-mass translation and relative rotation.

The system has a closed-form free solution. This allows us to test energy, both momentum maps, constraints, rotational covariance, and trajectory accuracy independently.

The scheme specializes the semi-explicit pHDAE discrete-gradient construction of Kinon, Morandin & Schulze (2026), *Applied Numerical Mathematics* 223, 45–75, DOI [10.1016/j.apnum.2025.12.006](https://doi.org/10.1016/j.apnum.2025.12.006). Their Remark 13 motivates G-equivariant discrete gradients for momentum-map preservation. The present implementation is a purpose-built two-body reference, not a general DAE solver and not a claim of novel mathematical discovery.

## Equations and exact discrete construction

Let q1,q2,v1,v2 ∈ R², masses m1,m2>0, link length L>0, and

\[
r=q_2-q_1,\qquad c(q_1,q_2)=\frac12(r^Tr-L^2)=0.
\]

The Hamiltonian and momentum maps are

\[
H=\frac12m_1v_1^Tv_1+\frac12m_2v_2^Tv_2,
\]

\[
P=m_1v_1+m_2v_2,
\qquad
J=m_1(q_1\times v_1)+m_2(q_2\times v_2).
\]

The equations with constant center-applied external forces f1,f2 and a link multiplier λ are

\[
m_1\dot v_1=f_1+\lambda r,\qquad
m_2\dot v_2=f_2-\lambda r,
\qquad
r^T(v_2-v_1)=0.
\]

Define the midpoint variables

\[
\bar q_i=\frac{q_{i,0}+q_{i,1}}2,\qquad
\bar v_i=\frac{v_{i,0}+v_{i,1}}2,\qquad
\bar r=\bar q_2-\bar q_1.
\]

The exact discrete Jacobian of the link constraint is the midpoint relative position \bar r. The update is

\[
q_{i,1}-q_{i,0}=h\bar v_i,
\]

\[
m_1(v_{1,1}-v_{1,0})=h(f_1+\lambda\bar r),
\]

\[
m_2(v_{2,1}-v_{2,0})=h(f_2-\lambda\bar r),
\]

\[
\bar r^T(\bar v_2-\bar v_1)=0.
\]

It follows directly that

\[
c(q_{1,1},q_{2,1})-c(q_{1,0},q_{2,0})
=h\bar r^T(\bar v_2-\bar v_1)=0.
\]

The discrete reaction impulse is equal and opposite, so it does not change total linear momentum. Its net torque about the origin also vanishes:

\[
\bar q_1\times(\lambda\bar r)
+\bar q_2\times(-\lambda\bar r)
=(\bar q_1-\bar q_2)\times(\lambda\bar r)=0.
\]

The kinetic-energy change is

\[
H_1-H_0
=h(f_1^T\bar v_1+f_2^T\bar v_2)
-h\lambda\bar r^T(\bar v_2-\bar v_1).
\]

The discrete algebraic constraint makes the reaction term zero, leaving the external-work identity

\[
H_1-H_0=h(f_1^T\bar v_1+f_2^T\bar v_2).
\]

For f1=f2=0, energy, total linear momentum, and total angular momentum are conserved to the nonlinear solve and floating-point envelope.

## Scalar multiplier reduction

Let r0=q2,0−q1,0, w0=v2,0−v1,0, and

\[
a=w_0+\frac h2\left(\frac{f_2}{m_2}-\frac{f_1}{m_1}\right),
\qquad
\alpha=\frac h2\left(\frac1{m_1}+\frac1{m_2}\right).
\]

Eliminating the two midpoint velocities gives

\[
\bar w(\lambda)=
\frac{a-\alpha\lambda r_0}
{1+\frac h2\alpha\lambda},
\qquad
F(\lambda)=r_0^T\bar w+\frac h2\bar w^T\bar w=0.
\]

The source evaluates F and its analytic derivative, uses Newton with residual-decreasing backtracking, and rejects singular or non-convergent solves. The continuous multiplier is only the initial guess; the accepted multiplier solves the discrete midpoint constraint.

## Symmetry and observable contract

Each accepted step reports independently:

- link-length residual and midpoint algebraic velocity-constraint residual;
- endpoint hidden-velocity residual;
- energy before/after, external work, reaction work, and energy-balance residual;
- total linear-momentum residual after the external impulse;
- angular-momentum residual after the external torque impulse;
- multiplier residual and iteration count.

The test suite also verifies:
- SO(2) rotational equivariance under a generic angle and rotated external forces;
- Galilean boost equivariance;
- equal- and unequal-mass cases;
- zero relative angular-velocity / pure translation control;
- nonzero center-of-mass translation plus relative rotation;
- exact analytic free dumbbell trajectory under timestep refinement;
- invalid link length, radial relative velocity, invalid/NaN masses, non-finite state/force, zero timestep, invalid solver options, and a deliberately under-iterated multiplier solve.

The method's endpoint hidden velocity constraint is measured explicitly. In the isolated torque-free dumbbell, the only relative acceleration is parallel to the midpoint link. That makes the endpoint tangent residual also vanish to roundoff in this special model. This special-case result must **not** be generalized to arbitrary semi-explicit pHDAEs: the broader published method can have only approximate endpoint hidden-velocity satisfaction for other systems.

## Independent equation-level numerical checks

The following were computed with a separate Python double-precision prototype of the equations. These are **not Rust implementation or CI results**.

### Long-horizon isolated dumbbell

Unequal masses m1=1.3 kg, m2=2.1 kg; L=1.2 m; initial center-of-mass velocity nonzero; relative angular velocity nonzero; 5,000 steps at h=0.002 s (10 s total):

| Observable | Maximum per-step residual | Final drift |
|---|---:|---:|
| Relative energy-balance residual | 1.63e-16 | Relative energy drift: 2.15e-14 |
| Link squared-length residual | — | 8.28e-14 |
| Midpoint velocity-constraint residual (relative) | — | 5.03e-15 |
| Total linear momentum | 4.97e-16 | 6.56e-15 |
| Angular momentum | 2.22e-15 | 1.31e-14 |
| Endpoint hidden velocity residual (absolute) | — | 9.69e-15 |

### Exact free-rotation trajectory convergence

The exact solution is rigid rotation of the relative vector at constant angular speed, plus uniform center-of-mass translation. Compared with that analytic solution at T=1 s:

| h (s) | Final state error | Successive error ratio |
|---:|---:|---:|
| 0.040 | 4.1384e-5 | — |
| 0.020 | 1.0347e-5 | 3.99968 |
| 0.010 | 2.5868e-6 | 3.99992 |
| 0.005 | 6.4670e-7 | 3.99998 |

This is second-order convergence for the frozen smooth case. It does not establish performance or physical validity.

## Competitive next steps

1. Run the exact Rust reference under the pinned workflow and compare its actual CSV to these independent numerical checks.
2. Add a separately coded Lagrange-multiplier / RATTLE baseline for the same two-body system.
3. Add a symmetry-breaking external force/torque campaign and verify exact momentum-map changes against signed impulses.
4. Extend to 3D, where the angular-momentum vector/bivector and SO(3) covariance become more demanding.
5. Add a small rigid tetrahedron or articulated chain and test whether momentum maps, constraint geometry, and work receipts remain composable.
6. Add Symtropy TGS and external-engine comparisons only when the same constraint and force semantics can be represented faithfully.

## Claim ceiling

This is a specialized two-point-mass, planar, fixed-link pHDAE reference. It does not implement a general rigid-body engine, 3D SO(3) solver, multiple-link chain, contact/impact, friction, soft bodies, adaptive stepping, sparse DAE infrastructure, or PhysicsWorld integration. No SOTA, production, or physical-validation claim is made.

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/phdae-dumbbell-reference/constrained_dumbbell.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/phdae-dumbbell-reference/constrained_dumbbell.rs -o /tmp/dumbbell-tests
/tmp/dumbbell-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/phdae-dumbbell-reference/constrained_dumbbell.rs -o /tmp/dumbbell-ref
/tmp/dumbbell-ref > /tmp/dumbbell-a.csv
/tmp/dumbbell-ref > /tmp/dumbbell-b.csv
cmp /tmp/dumbbell-a.csv /tmp/dumbbell-b.csv
~~~
