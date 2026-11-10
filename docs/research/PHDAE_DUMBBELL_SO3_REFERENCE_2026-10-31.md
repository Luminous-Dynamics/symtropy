# Spatial SO(3) constrained dumbbell pHDAE discrete-gradient reference

**Status:** experimental 3D symmetry/momentum-map reference; no PhysicsWorld integration.  
**Date:** 2026-10-31  
**Research parent:** [#3132 PHYS-PHDAE-SO3-001](https://github.com/Luminous-Dynamics/symtropy/issues/3132).  
**Parent method:** [#3120 planar pHDAE dumbbell](https://github.com/Luminous-Dynamics/symtropy/pull/3120).  
**Shared qualification owner:** [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Why this is a meaningful 3D step

The planar dumbbell tests SO(2) covariance and scalar angular momentum. This implementation moves the same discrete mechanics to three spatial coordinates:
- arbitrary 3D link direction;
- relative velocity with all three components nonzero;
- 3D cross-product torque;
- full angular-momentum vector;
- generic SO(3) rotations;
- Rodrigues exact free-motion reference.

This is still a two-point-mass rigid-link model. It is not a general rigid-body orientation integrator or articulated multibody engine.

## Model

Let q1,q2,v1,v2 ∈ R³, masses m1,m2>0, link length L>0, relative position r=q2−q1, and relative velocity w=v2−v1.

The holonomic constraint and kinetic Hamiltonian are

\[
c(q_1,q_2)=\frac12(r^Tr-L^2)=0,
\qquad
H=\frac12m_1v_1^Tv_1+\frac12m_2v_2^Tv_2.
\]

The conserved momentum maps of the isolated system are

\[
P=m_1v_1+m_2v_2\in\mathbb R^3,
\]

\[
J=m_1(q_1\times v_1)+m_2(q_2\times v_2)\in\mathbb R^3.
\]

For external center-applied forces f1,f2 and link multiplier λ:

\[
m_1\dot v_1=f_1+\lambda r,\qquad
m_2\dot v_2=f_2-\lambda r,\qquad
r^T(v_2-v_1)=0.
\]

## Discrete construction

Define midpoint coordinates q̄_i=(q_i,0+q_i,1)/2 and velocities v̄_i=(v_i,0+v_i,1)/2. The exact discrete Jacobian of the squared-distance constraint is the midpoint relative vector

\[
\bar r=\bar q_2-\bar q_1=\frac{r_0+r_1}{2}.
\]

The semi-explicit pHDAE discrete-gradient step is

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
\bar r^T\bar w=0,\qquad \bar w=\bar v_2-\bar v_1.
\]

The discrete Jacobian identity gives

\[
c(q_{1,1},q_{2,1})-c(q_{1,0},q_{2,0})
=h\bar r^T\bar w=0.
\]

The reaction impulses are equal and opposite, so they preserve total linear momentum. Their torque vector is

\[
\bar q_1\times(\lambda\bar r)
+\bar q_2\times(-\lambda\bar r)
=(\bar q_1-\bar q_2)\times(\lambda\bar r)=0.
\]

The energy identity is

\[
H_1-H_0
=h(f_1^T\bar v_1+f_2^T\bar v_2)
-h\lambda\bar r^T\bar w
=W_{ext}.
\]

No post-hoc projection or energy correction is applied.

The scalar multiplier reduction is the same as in the planar dumbbell, with vector dot products in R³:

\[
\bar w(\lambda)=
\frac{w_0+\frac h2(f_2/m_2-f_1/m_1)-\alpha\lambda r_0}
{1+\frac h2\alpha\lambda},
\quad
\alpha=\frac h2\left(\frac1{m_1}+\frac1{m_2}\right),
\]

\[
F(\lambda)=r_0^T\bar w+\frac h2\bar w^T\bar w=0.
\]

The source evaluates F and its analytic derivative, then uses Newton with residual-decreasing backtracking. Singular and non-convergent solves fail closed.

## Independent exact solution: Rodrigues rotation

For an isolated dumbbell, the center of mass moves uniformly. The relative link rotates in the plane orthogonal to the constant angular-velocity vector

\[
\omega=\frac{r_0\times w_0}{L^2}.
\]

For rotation vector \(\phi=\omega t\), Rodrigues' formula is

\[
R(\phi)x
=x\cos\theta
+(\hat\phi\times x)\sin\theta
+\hat\phi(\hat\phi^Tx)(1-\cos\theta),
\qquad \theta=\|\phi\|.
\]

Then r(t)=R(ωt)r0 and w(t)=R(ωt)w0, with the individual mass positions and velocities recovered by mass-weighted center-of-mass coordinates. The implementation includes a small-angle series branch to avoid normalizing a near-zero rotation vector.

This exact solution is independent of the pHDAE multiplier and time-stepping equations.

## Frozen 3D fixture

- m1=1.3 kg, m2=2.1 kg, L=1.2 m.
- Center position R=(0.25,-0.12,0.30) m.
- Center velocity V=(0.17,-0.23,0.11) m/s.
- Relative link r=(0.8,0.8,0.4) m.
- Relative velocity w=(0.2,-0.1,-0.2) m/s.
- r·w=0, while all components of r, w, and ω=r×w/L² are nonzero.
- Isolated free run: 5,000 steps, h=0.002 s, T=10 s.
- Convergence run: T=1 s; h=0.04, 0.02, 0.01, 0.005 s.

## Implemented receipts and tests

The dependency-free Rust source is:

tools/phdae-dumbbell-3d-reference/constrained_dumbbell_3d.rs

It records:
- link-length and midpoint algebraic constraint residuals;
- endpoint hidden-velocity residual;
- Hamiltonian, external work, constraint reaction work and energy-balance residual;
- linear-momentum residual vector;
- full angular-momentum residual vector;
- scalar multiplier, raw/scaled solve residual and iteration count.

Tests cover:
- genuinely 3D fixture (nonzero x/y/z components);
- long-horizon link, midpoint and endpoint constraints;
- energy, linear momentum and angular momentum vector preservation;
- second-order convergence against exact Rodrigues motion;
- twenty deterministic generic axis-angle SO(3) metamorphic rotations, with external forces rotated along with the state;
- Galilean boost covariance in 3D;
- unequal external force work and vector impulse/torque ledger;
- small-angle Rodrigues stability and zero-relative-spin pure translation;
- invalid link, radial relative velocity, non-finite state/force, invalid masses, zero timestep, invalid solver options and under-iterated multiplier solve.

## Independent equation-level numerical checks

These are Python double-precision equation-level checks, not Rust CI results.

### Long-horizon isolated case

| Observable | Maximum relative residual over the run |
|---|---:|
| Link squared-length residual | 2.92e-15 |
| Midpoint algebraic constraint | 2.87e-15 |
| Endpoint hidden velocity constraint | 3.90e-15 |
| Energy drift | 1.24e-15 |
| Linear momentum vector drift | 2.57e-15 |
| Angular momentum vector drift | 6.82e-15 |

### Exact Rodrigues trajectory convergence

| h (s) | Final state error |
|---:|---:|
| 0.040 | 1.8719e-6 |
| 0.020 | 4.6798e-7 |
| 0.010 | 1.1700e-7 |
| 0.005 | 2.9249e-8 |

The error contracts by approximately four per timestep halving, consistent with second-order convergence for the frozen problem.

### Unequal external force one-step control

With f1=(0.12,-0.07,0.05) and f2=(-0.03,0.09,0.11) N at h=0.01 s, an independent prototype gave:
- energy change: 1.3170488799563e-5 J;
- external work: 1.3170488799575e-5 J;
- constraint reaction work: -9.62e-18 J;
- energy-balance defect: -2.49e-18 J;
- linear impulse: (0.0009,0.0002,0.0016) kg·m/s, matching h(f1+f2);
- angular impulse vector: (-0.00047441,-0.00056188,0.00146543) kg·m²/s, matching h(q̄1×f1+q̄2×f2).

These are independent equation-level cross-checks, not output from the committed Rust implementation.

## Competitive next steps

1. Add a 3D Cartesian RATTLE baseline and a matched pHDAE/RATTLE campaign.
2. Compare 3D pHDAE with a 3D constrained multibody reference only after the same joint/link semantics are represented exactly.
3. Add a small articulated tetrahedron or three-link spatial chain; measure the full vector angular momentum map and SO(3) covariance.
4. Extend to general rigid-body orientation, where quaternion/manifold integration and Lie-group variational methods become necessary. Do not conflate that with this two-point-mass constraint.
5. Feed all machine-readable scenario receipts into #460.

## Claim ceiling

This is a 3D, two-point-mass, fixed-link pHDAE reference. It does not implement general rigid-body orientation, arbitrary multibody constraints, articulated chains, contact/impact, friction, soft bodies, adaptive stepping, sparse DAE infrastructure or PhysicsWorld integration. No SOTA, production or physical-validation claim is made.

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/phdae-dumbbell-3d-reference/constrained_dumbbell_3d.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/phdae-dumbbell-3d-reference/constrained_dumbbell_3d.rs -o /tmp/dumbbell-3d-tests
/tmp/dumbbell-3d-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/phdae-dumbbell-3d-reference/constrained_dumbbell_3d.rs -o /tmp/dumbbell-3d
/tmp/dumbbell-3d > /tmp/dumbbell-3d-a.csv
/tmp/dumbbell-3d > /tmp/dumbbell-3d-b.csv
cmp /tmp/dumbbell-3d-a.csv /tmp/dumbbell-3d-b.csv
~~~
