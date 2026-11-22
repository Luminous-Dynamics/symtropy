# 5D EGB solution-branch atlas: continuation and bifurcation protocol

**Owner:** [#6333 BH-5D-EGB-BRANCH-ATLAS-001](https://github.com/Luminous-Dynamics/symtropy/issues/6333)

**Purpose:** compute the topology of the stationary equal-spin 5D EGB solution set, rather than only a sequence of solutions at fixed coupling values.

## Why a branch atlas is needed

A parameter sweep solves
\[
\mathcal F_N(U;\lambda,\omega)=0.
\]
It can fail when the solution curve turns around in \(\lambda\), and it does not automatically discover branches disconnected from the initial solution.

AUTO-07p and BifurcationKit are mature references for continuation and bifurcation. BifurcationKit also supports pseudo-arclength, bordered solvers, event detection and deflated continuation. Symtropy should not claim to invent these algorithms. Its advance is to make them gravity-aware and evidence-carrying: preserve Lovelock constraints, horizon/gauge contracts, invariant-observable branch matching and independent candidate qualification.

## Frozen state and metric

Write the discretized problem as
\[
F_N(U,\lambda;\omega)=0,
\qquad \lambda=\alpha/r_H^2,\quad \omega=r_H\Omega_H.
\]
Initially fix \(r_H=1\), \(\omega=0.33\), and continue in \(\lambda\) to match the published 5D EGB benchmark.

Define an arclength norm using spectral quadrature weights, explicit physical scales for the field coefficients and separate scales for dimensionless parameters. Freeze this metric for each benchmark run. A units or parameter-scale change must not silently change the physical branch.

## Pseudo-arclength corrector

At a known branch point \((U_0,\lambda_0)\), predict
\[
(U_p,\lambda_p)=(U_0,\lambda_0)+\Delta s(t_U,t_\lambda)
\]
using a normalized tangent. Correct by solving
\[
\begin{cases}
F_N(U,\lambda;\omega)=0,\\
\langle t_U,U-U_p\rangle_W+
\gamma_\lambda t_\lambda(\lambda-\lambda_p)=0.
\end{cases}
\]
The Newton corrector uses the bordered matrix
\[
\begin{pmatrix}
F_U & F_\lambda\\
t_U^TW_U & \gamma_\lambda t_\lambda
\end{pmatrix}.
\]
This avoids requiring \(F_U^{-1}\) at a simple fold. Adapt the step using corrector iteration count, predictor/corrector discrepancy, branch curvature and bordered-system conditioning.

A two-parameter surface in \((\lambda,\omega)\) requires a codimension-two continuation formulation; do not alternate parameters heuristically and call the result a surface.

## Event detection

Record sign changes in the parameter tangent, refined fold locations, smallest singular values and approximate left/right kernel vectors, bordered-system conditioning, kernel dimension and transversality/normal-form tests.

A small singular value alone does not prove a bifurcation. Distinguish simple folds, genuine branch points, gauge/coordinate degeneracies, horizon regularity failure, numerical corrector failure and physical admissibility boundaries.

## Deflation for disconnected solutions

At selected fixed parameter values, use a deflation operator to discourage Newton iteration from rediscovering known roots. Use physically scaled solution distances and deterministic, recorded initial perturbations.

Deflation is a complementary search method, not a proof of completeness. A deflated candidate must pass the original, undeflated equations, all constraints, horizon and asymptotic conditions, and independent qualification. Retain unsuccessful searches and the perturbation/search budget.

## Exact numerical controls

### Fold
\[
F(u,\lambda)=u^2-\lambda=0,\qquad \lambda=u^2.
\]
The curve turns at \((u,\lambda)=(0,0)\). Natural continuation in \(\lambda\) cannot traverse the complete connected curve; pseudo-arclength should continue from \(u>0\) through zero to \(u<0\).

### Cusp
\[
F(u;\mu,\nu)=u^3+\mu u+\nu=0.
\]
The discriminant \(4\mu^3+27\nu^2=0\) is the exact fold locus. Use it to test two-parameter event localization.

### Multiple roots
Use a frozen polynomial with distinct known roots to test deflation, duplicate rejection and original-residual requalification. Finite successful initial guesses test the implementation, not completeness in general.

## Black-hole benchmark

First reproduce the convention-matched Akribeia/2010 5D equal-spin EGB case at \(r_H=1\), \(\omega=0.33\), and \(\lambda\in[0,3]\), subject to exact reference-array and boundary-convention extraction under #6063.

Compare natural stepping and pseudo-arclength on identical discretizations, tolerances and starting solutions. Record branch coverage, event locations, residuals, radial constraint and Noether-identity defects, spectral tails, boundary errors, physical-observable continuity, rejected steps, runtime and memory.

Continuation success is not a physical validity certificate.

## Branch graph and evidence

Represent the atlas as a graph. Nodes are exact controls, accepted continuation points, folds, branch points, cross-gauge matches and endpoints. Edges are oriented continuation segments with parameter intervals, tangent data, method, discretization and evidence digests.

Branch identity must be invariant under radial-gauge changes and remeshing. Store coordinate profiles for replay, but use common-frame invariant observables and a declared normalized metric for candidate deduplication.

Classify endpoints separately: extremality/zero surface gravity, horizon-series degeneracy, spectral failure, loss of hyperbolicity, singular observable or unresolved numerical termination.

## References and claim boundaries

- AUTO-07p: https://auto-07p.github.io/
- BifurcationKit PALC: https://bifurcationkit.github.io/BifurcationKitDocs.jl/dev/PALC/
- BifurcationKit deflated continuation: https://bifurcationkit.github.io/BifurcationKitDocs.jl/dev/DeflatedContinuation/
- Farrell, Beentjes & Birkisson, The Computation of Disconnected Bifurcation Diagrams, arXiv:1603.00809.
- Farrell, Computing Multiple Solutions of Systems of Nonlinear Equations with Deflation, ICM 2026, DOI 10.1137/25M1805710.
- Akribeia, accepted PRD 12 Nov 2026: https://doi.org/10.1103/3kz2-6dvv.

No folds, disconnected branches or bifurcations in the target EGB family are presumed. No completeness claim is made without an independent completeness argument.


## Observable projection overlap is not a branch fold

A published 5D EGB reference family can be non-monotonic in a selected profile observable. A repeated value of \(f(r_*)\), or even a visual crossing in a two-dimensional plot, does not imply that the full BVP solution manifold folds or has multiple roots at a fixed parameter.

A new exact-rational synthetic control freezes this distinction:

\[
F((u,v),\lambda)=
\begin{pmatrix}
u-\lambda\\
v-\lambda^2
\end{pmatrix}=0,
\qquad U(\lambda)=(\lambda,\lambda^2).
\]

The observable \(O(U)=v\) takes the same value \(1/4\) at \(\lambda=-1/2\) and \(+1/2\), although the full states differ. The fixed-parameter state Jacobian is \(D_UF=I\), with determinant 1, and \(\lambda\) is a regular branch parameter. The correct classification is **observable projection self-overlap**, not a BVP fold.

The distinct exact fold control remains
\[
F(u,\lambda)=u^2-\lambda=0,\qquad \lambda=u^2,
\]
whose solution curve has \(d\lambda/du=0\) and \(d^2\lambda/du^2=2\) at the origin, with the fixed-parameter derivative \(F_u=0\) and \(F_\lambda=-1\).

The new dependency-free verifier is \`branch_atlas_projection_overlap_reference.py\`. It is an exact synthetic control only; it does not reproduce the Akribeia 5D family or identify a target EGB event. Its purpose is to prevent scalar projections from being promoted to branch-topology claims without the full solution Jacobian and transversality tests.
