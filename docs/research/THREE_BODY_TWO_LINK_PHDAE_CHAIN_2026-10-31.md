# Coupled 3-body / 2-link spatial pHDAE discrete-gradient chain

**Status:** experimental coupled multi-constraint pHDAE reference; open chain only.  
**Date:** 2026-10-31  
**Research parent:** [#3141 PHYS-MULTICONSTRAINT-001](https://github.com/Luminous-Dynamics/symtropy/issues/3141).  
**Related spatial single-link work:** [#3134 pHDAE dumbbell](https://github.com/Luminous-Dynamics/symtropy/pull/3134), [#3138 matched pHDAE/RATTLE](https://github.com/Luminous-Dynamics/symtropy/pull/3138).  
**Shared qualification owner:** [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Why this is a different numerical problem

A single dumbbell requires one scalar holonomic multiplier. A three-body open chain has two coupled distance constraints. Each link multiplier affects the shared middle body's velocity, which in turn changes both midpoint link vectors. Solving two scalar link equations independently is therefore incorrect.

This implementation treats the chain as one constraint graph:
- three point masses;
- two directed link edges (0,1) and (1,2);
- two holonomic constraints;
- a coupled 9×9 midpoint-velocity block system;
- a coupled 2×2 nonlinear multiplier solve.

The goal is not a generic multibody claim. It is the first small model where a coupled multiplier vector and its conditioning must be visible in every step receipt.

## Model

For body positions \(q_i\in\mathbb R^3\), velocities \(v_i\), masses \(m_i>0\), and two links \(e=(i,j)\) with lengths \(L_e>0\), define

\[
g_e(q)=\frac12(\|q_j-q_i\|^2-L_e^2)=0.
\]

The kinetic Hamiltonian and conserved momentum maps of the isolated chain are

\[
H(q,v)=\frac12\sum_{i=0}^2m_i v_i^Tv_i,
\]

\[
P=\sum_i m_i v_i,\qquad
J=\sum_i m_i(q_i\times v_i)\in\mathbb R^3.
\]

With a midpoint link vector \(\bar r_e=\bar q_j-\bar q_i\), the continuous equations with external center-applied forces \(f_i\) are

\[
m_i\dot v_i=f_i+\sum_{e\sim i}B_{ie}\lambda_e\bar r_e,
\]

where B is +1 at the edge's from body and -1 at its to body. The sign convention is fixed in the implementation and used consistently in the work and momentum ledgers.

## Coupled discrete-gradient step

For each body,

\[
q_{i,1}-q_{i,0}=h\bar v_i,
\]

\[
m_i(v_{i,1}-v_{i,0})
=h\left(f_i+\sum_e B_{ie}\lambda_e\bar r_e\right),
\]

with

\[
\bar q_i=\frac{q_{i,0}+q_{i,1}}2,\quad
\bar v_i=\frac{v_{i,0}+v_{i,1}}2,\quad
\bar r_e=\bar q_j-\bar q_i.
\]

For each edge, the exact discrete Jacobian is its midpoint relative position, and the algebraic constraint is

\[
F_e(\lambda)=\bar r_e^T\bar w_e=0,\qquad
\bar w_e=\bar v_j-\bar v_i.
\]

Using the discrete-Jacobian identity,

\[
g_e(q_1)-g_e(q_0)
=h\bar r_e^T\bar w_e=0.
\]

Thus both link position constraints propagate from a valid initial state to the solver/roundoff envelope. The constraint reaction work is

\[
W_c=-h\sum_e\lambda_e\bar r_e^T\bar w_e=0.
\]

Each link reaction is equal and opposite, and its torque cancels because both forces act along the same midpoint link. The energy identity is therefore

\[
H_1-H_0=h\sum_i f_i^T\bar v_i.
\]

No post-step position projection or energy correction is used.

## Coupled 9×9 midpoint-velocity block

For fixed lambda=(lambda0,lambda1), the implementation eliminates the midpoint positions and builds a dense 9×9 system for the nine midpoint-velocity components.

Each edge multiplier contributes to both endpoint rows. Since the middle body belongs to both edges, its diagonal block receives both link couplings. This is the key coupling that an edge-by-edge scalar update would miss.

The block matrix is solved with partial pivoting. A pivot below the declared relative threshold returns a typed singular-system failure; the code does not silently add diagonal regularization.

## Coupled 2×2 multiplier Newton solve

The discrete algebraic residual vector is

\[
F(\lambda)=\begin{bmatrix}F_0(\lambda)\\F_1(\lambda)\end{bmatrix}
=\begin{bmatrix}\bar r_0^T\bar w_0\\\bar r_1^T\bar w_1\end{bmatrix}=0.
\]

The implementation forms the exact 2×2 Jacobian DF(lambda) by differentiating the 9×9 block system with respect to each multiplier:
- differentiate the block matrix and right-hand side;
- solve the differentiated block system for each multiplier column;
- evaluate the derivative of both link residuals;
- solve the coupled 2×2 Newton correction;
- accept only a residual-decreasing backtracked step.

The continuous acceleration-level multiplier vector is used only as the initial guess. The accepted multiplier vector solves the discrete midpoint equations.

Each accepted step records:
- both multipliers and both residuals;
- nonlinear iterations;
- absolute determinant of the multiplier Jacobian;
- normalized determinant |det J|/(||J0|| ||J1||), a scale-free row-independence indicator;
- each link's position, midpoint, and endpoint hidden-velocity residuals;
- energy, external work, constraint reaction work, linear momentum, and angular-momentum-vector residuals.

A near-zero normalized determinant returns SingularConstraintJacobian. This is a diagnostic for the two-constraint open chain; it is not yet a general rank-revealing QR/SVD certificate for arbitrary constraint graphs.

## Frozen open-chain fixture

- Masses: (1.3, 2.1, 0.8) kg.
- Positions: q0=(-0.45,0.10,0.30), q1=(0.25,0.60,0.60), q2=(0.05,1.40,1.10) m.
- Relative link vectors: (0.70,0.50,0.30) m and (-0.20,0.80,0.50) m.
- Unequal link lengths, derived from the frozen geometry.
- Nonzero center-of-mass translation and both link-relative velocities.
- Isolated horizon: 2,000 steps at h=0.002 s (4 s total).
- Temporal-order ladder: h=0.02, 0.01, 0.005, 0.0025 s over T=1 s.

The tests separately include equal-mass and unequal/non-parallel external-force cases, 20 generic SO(3) rotations with force vectors rotated along with the chain, and Galilean boost covariance.

## Independent continuous reference

The temporal-order test uses a separate continuous constrained ODE reference. At each RK4 stage it solves the acceleration-level 2×2 system

\[
A_{ef}(q)\lambda_f
=-(\|\bar w_e\|^2+\bar r_e^T(a_{ext,j}-a_{ext,i})),
\]

where each entry is built directly from the instantaneous link vectors and inverse masses. It then evaluates body accelerations from the continuous Lagrange-multiplier equations.

This reference does not reuse the discrete midpoint block system or its Newton Jacobian. Its RK4 integration uses 40,000 uniform steps over one second.

## Independent equation-level results

These are separate double-precision prototype checks, **not Rust CI or committed-runner results**.

### Long-horizon conservation and constraint closure

For 2,000 steps at h=0.002 s:

| Observable | Maximum / final result |
|---|---:|
| Per-step relative energy-balance residual | 1.83e-16 |
| Maximum constraint-reaction work magnitude | 2.85e-20 J |
| Link 0 maximum relative position residual | 5.43e-16 |
| Link 1 maximum relative position residual | 2.48e-15 |
| Link 0 maximum relative midpoint residual | 1.19e-16 |
| Link 1 maximum relative midpoint residual | 1.32e-16 |
| Link 0 maximum relative endpoint hidden-velocity residual | 5.16e-9 |
| Link 1 maximum relative endpoint hidden-velocity residual | 1.03e-8 |
| Relative energy drift | 1.31e-14 |
| Relative linear-momentum drift | 6.58e-15 |
| Relative angular-momentum-vector drift | 5.76e-15 |
| Minimum normalized multiplier-Jacobian determinant | 0.889 |
| Maximum Newton updates per step | 1 |

Endpoint hidden-velocity residuals are reported separately; they are not the algebraic midpoint constraints enforced by the method. The prototype's upper link endpoint residual motivates the current 2e-8 test ceiling rather than falsely claiming exact endpoint tangency.

### Temporal convergence against continuous constrained RK4

| h (s) | Final state error | Successive ratio |
|---:|---:|---:|
| 0.0200 | 8.2139e-7 | — |
| 0.0100 | 2.0535e-7 | 3.99999 |
| 0.0050 | 5.1337e-8 | 4.00000 |
| 0.0025 | 1.2834e-8 | 4.00000 |

The coarser 0.04 s step is intentionally not used for the observed-order acceptance because this model has not yet entered its asymptotic error regime there.

### Unequal external-force one-step control

With
- f0=(0.12,-0.07,0.05) N,
- f1=(-0.03,0.09,0.11) N,
- f2=(0.04,0.02,-0.08) N,
- h=0.01 s,

the independent prototype gives:
- energy change: 3.5325236681133e-4 J;
- external work: 3.5325236681145e-4 J;
- constraint reaction work: 3.46e-18 J;
- energy-balance defect: -1.24e-16 J;
- linear impulse: (0.0013,0.0004,0.0008) kg·m/s, matching h sum(f_i);
- angular impulse vector: (-9.6048e-4,6.1186e-4,5.2341e-5) kg·m²/s, matching h sum(qbar_i×f_i).

These are equation-level checks, not Rust implementation results.

## Current scope and next rung

This reference is an open spatial chain with exactly two independent distance constraints. It does not yet handle:
- closed loops or redundant constraints;
- arbitrary graph topology;
- general QR/SVD rank-revealing solves;
- compliant joints, rigid-body attitude or articulated body inertia;
- contact, impact, friction, adaptive stepping or PhysicsWorld integration.

The next child should add a closed triangle with a controlled near-collinear geometry ladder. It must classify rank-deficient/inconsistent constraints explicitly and must not regularize them into an apparently valid solve.

## Claim ceiling

This is a research implementation of a three-point-mass, two-link open chain. It is not a general multibody engine, not a general pHDAE descriptor solver, and not a novel mathematical method claim. No SOTA, physical-validation, production or industry-superiority claim is made.

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/phdae-threebody-chain/three_body_chain.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/phdae-threebody-chain/three_body_chain.rs -o /tmp/threebody-chain-tests
/tmp/threebody-chain-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/phdae-threebody-chain/three_body_chain.rs -o /tmp/threebody-chain
/tmp/threebody-chain > /tmp/threebody-chain-a.csv
/tmp/threebody-chain > /tmp/threebody-chain-b.csv
cmp /tmp/threebody-chain-a.csv /tmp/threebody-chain-b.csv
~~~
