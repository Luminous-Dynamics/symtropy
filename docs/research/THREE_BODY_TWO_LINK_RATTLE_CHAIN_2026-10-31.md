# Independent Cartesian SHAKE/RATTLE baseline for the spatial three-body open chain

**Status:** experimental independent RATTLE reference; isolated conservative chain only.  
**Date:** 2026-10-31  
**pHDAE open-chain comparator:** [#3144](https://github.com/Luminous-Dynamics/symtropy/pull/3144).  
**Rank preflight:** [#3157](https://github.com/Luminous-Dynamics/symtropy/pull/3157).  
**Research programme:** [#3153 PHYS-CONSTRAINT-GRAPH-001](https://github.com/Luminous-Dynamics/symtropy/issues/3153).  
**Qualification authority:** [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Why a separate RATTLE baseline

The coupled pHDAE chain in #3144 solves a 9×9 midpoint-velocity block and a coupled 2×2 nonlinear multiplier problem. This PR adds a different algorithm: Cartesian SHAKE position correction followed by a distinct RATTLE endpoint-velocity projection.

It does not reuse the pHDAE midpoint block, residual Jacobian or multiplier iteration. Both methods can now be compared against the same continuous constrained equations and initial state.

## Model

Three point masses in R³, connected by two massless fixed-length links:
- m=(1.3,2.1,0.8) kg;
- edges e0=(0,1), e1=(1,2);
- fixed lengths are derived from the frozen initial positions;
- no potential or external force in this v0.1 reference.

For each edge e=(i,j),

\[
g_e(q)=\frac12(\|q_j-q_i\|^2-L_e^2)=0.
\]

The position and velocity constraints are both independent per edge. The solver uses the mass-weighted geometric Gram matrix \(GM^{-1}G^T\) for a fail-closed rank preflight before attempting a step.

## SHAKE position stage

The free drift is

\[
q^*=q_n+h v_n.
\]

With position multipliers \(\lambda_e\), the corrected endpoint positions are

\[
q_{n+1}=q^*-\frac{h^2}{2}M^{-1}G(q_n)^T\lambda.
\]

Both endpoint constraints are solved simultaneously:

\[
g(q_{n+1}(\lambda))=0.
\]

The implementation forms the exact 2×2 Newton Jacobian

\[
J_{ea}=-\frac{h^2}{2}
G_e(q_{n+1})M^{-1}G_a(q_n)^T
\]

and uses residual-decreasing backtracking. No diagonal-per-edge approximation and no sequential link projection are used. A rank-deficient or non-convergent solve fails closed.

## RATTLE endpoint velocity stage

The half-step momentum is

\[
p_{n+1/2}=p_n-\frac h2G(q_n)^T\lambda.
\]

The separate endpoint correction is

\[
p_{n+1}=p_{n+1/2}-\frac h2G(q_{n+1})^T\mu,
\]

where the coupled 2×2 mass-weighted Gram system chooses \(\mu\) so

\[
G(q_{n+1})M^{-1}p_{n+1}=0.
\]

The position correction and endpoint velocity projection are therefore distinct operations with distinct multiplier vectors. This is a true RATTLE-style endpoint tangent solve, not a midpoint tangent constraint.

## Receipts

The source records:
- SHAKE position multipliers, Newton iterations/backtracks, residual vector and Jacobian rcond;
- RATTLE endpoint velocity multipliers;
- geometric constraint rank/rcond before and after the step;
- each edge's signed and relative position residual;
- each edge's midpoint diagnostic and endpoint tangent residual;
- Hamiltonian before/after and conservative energy defect;
- linear momentum residual vector and full angular-momentum residual vector.

The midpoint constraint is measured but not imposed. The endpoint position and endpoint tangent constraints are imposed by the two separate RATTLE stages. Energy defect is a numerical conservative-energy error, not physical dissipation and not post-corrected.

## Frozen fixture and reference

The initial state is the same spatial three-body chain fixture used in #3144:
- q0=(-0.45,0.10,0.30), q1=(0.25,0.60,0.60), q2=(0.05,1.40,1.10) m;
- v0=(0.0723809524,-0.2628571429,0.3131746032), v1=(0.1723809524,-0.2428571429,0.0465079365), v2=(0.3223809524,-0.1428571429,-0.0534920635) m/s;
- T=1 s, h=0.02, 0.01, 0.005, 0.0025 s for convergence;
- T=4 s, h=0.002 s for the 2,000-step invariant campaign.

The independent reference is a separately implemented acceleration-level constrained ODE. At every reference stage it solves the continuous 2×2 constraint Gram system to enforce the second derivative of each holonomic constraint, then integrates the resulting smooth ODE with RK4 at dt=1/40000 s. It does not call either RATTLE's SHAKE correction or the pHDAE discrete update.

## Independent equation-level checks

These are separate Python double-precision checks of the stated RATTLE equations, **not** output from the committed Rust source or the Rust workflow.

### Second-order convergence at T=1 s

| h (s) | final Cartesian state error |
|---:|---:|
| 0.0200 | 1.6023706e-6 |
| 0.0100 | 4.0058859e-7 |
| 0.0050 | 1.0014689e-7 |
| 0.0025 | 2.5036709e-8 |

Each timestep halving reduces the error by approximately 4.0, consistent with second-order convergence.

### 2,000-step invariant campaign at h=0.002 s

| Observable | Maximum / terminal result |
|---|---:|
| Link 0 relative position residual | 2.09e-16 |
| Link 1 relative position residual | 2.33e-16 |
| Link 0 endpoint tangent residual | 8.24e-17 |
| Link 1 endpoint tangent residual | 1.12e-16 |
| Midpoint diagnostic link 0 | 3.78e-9 |
| Midpoint diagnostic link 1 | 6.07e-9 |
| Relative energy drift | 3.42e-10 |
| Relative linear momentum drift | 1.93e-15 |
| Relative angular momentum-vector drift | 2.76e-15 |
| Maximum SHAKE Newton updates | 1 |

This illustrates the method distinction: endpoint tangency is enforced at the new position; the midpoint constraint is a diagnostic, not a RATTLE requirement. The energy drift is measured honestly and is larger than the pHDAE discrete energy-balance residual. The method is not energy-exact.

## Implemented tests

- both link positions and endpoint tangent constraints over 2,000 steps;
- linear and full angular-momentum-vector preservation;
- second-order convergence against the independent constrained-RK4 trajectory;
- 20 deterministic generic SO(3) rotations;
- Galilean boost covariance;
- invalid geometry, non-finite state, invalid masses and zero timestep;
- open-chain collinearity control, demonstrating that collinear open-chain links still have two independent constraints;
- geometric rank preflight on every step.

## Comparison protocol

The fair next campaign must run #3144 pHDAE and this RATTLE baseline from exactly the same initial state, masses, edge ordering, link lengths, timestep ladder and one-second horizon. It should report:
- final trajectory error against the common independent continuous RK4 reference;
- energy drift and pHDAE discrete energy-balance residual separately;
- each link's position/midpoint/endpoint residual;
- linear/angular momentum vector drift;
- pHDAE multiplier iterations versus RATTLE SHAKE Newton iterations and endpoint projection solve;
- geometric Gram rank/rcond and discrete multiplier-Jacobian conditioning separately;
- p50/p95/p99 runtime only after exact-source optimized binaries are benchmarked on identical hardware.

A matched campaign PR will follow this baseline. No external-engine result should be described as ground truth. The canonical differential runner remains [#443](https://github.com/Luminous-Dynamics/symtropy/issues/443); external comparisons should wait until the same two-link distance constraints and solver semantics are represented faithfully. The 3D friction-disk PR #3145 is not an open-chain RATTLE baseline.

## Claim ceiling

This is an independent Cartesian SHAKE/RATTLE reference for a three-point-mass, two-link, free spatial chain. It does not yet implement external force histories, a closed triangle, arbitrary constraint graphs, contact, impact, friction, general rigid-body attitude, production integration or SOTA superiority.

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/constraint-graph-rank/constraint_graph_rank.rs
rustup run 1.96.0 rustfmt --edition 2021 --check tools/rattle-threebody-chain/cartesian_rattle_chain.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/rattle-threebody-chain/cartesian_rattle_chain.rs -o /tmp/rattle-chain-tests
/tmp/rattle-chain-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/rattle-threebody-chain/cartesian_rattle_chain.rs -o /tmp/rattle-chain
/tmp/rattle-chain > /tmp/rattle-chain-a.csv
/tmp/rattle-chain > /tmp/rattle-chain-b.csv
cmp /tmp/rattle-chain-a.csv /tmp/rattle-chain-b.csv
~~~
