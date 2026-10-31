# Rank-aware geometric constraint graph diagnostics

**Status:** experimental geometric rank/conditioning preflight; not yet wired into the time-step solve.  
**Date:** 2026-10-31  
**Open-chain pHDAE reference:** [#3144](https://github.com/Luminous-Dynamics/symtropy/pull/3144).  
**Research programme:** [#3153 PHYS-CONSTRAINT-GRAPH-001](https://github.com/Luminous-Dynamics/symtropy/issues/3153).  
**Shared qualification authority:** [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Why the graph needs its own rank certificate

The open-chain pHDAE in #3144 solves a coupled 9×9 midpoint-velocity block and a coupled 2×2 nonlinear multiplier system. Its normalized multiplier-Jacobian determinant is a useful local singularity indicator, but it answers a different question from the rank of the *geometric constraint manifold*.

For bodies with generalized coordinates q and distance constraints

\[
g_e(q)=\frac12(\|q_j-q_i\|^2-L_e^2)=0,
\]

let G(q) be the full constraint Jacobian and M the block-diagonal mass matrix. The mass-weighted Gram matrix is

\[
C(q)=G(q)M^{-1}G(q)^T.
\]

The numerical rank and spectrum of C describe the independent constraint directions at the current geometry. The discrete multiplier Jacobian is timestep- and integrator-dependent. They must be reported separately.

A regular Newton solve at one timestep cannot certify that a redundant or rank-deficient constraint set has unique multipliers.

## Implemented module

The dependency-free Rust reference is **tools/constraint-graph-rank/constraint_graph_rank.rs**. It supports three point masses in R³ with two or three distance edges.

It provides:
- stable body IDs and edge IDs;
- deterministic edge ordering by stable edge ID;
- rejection of duplicate body/edge IDs, self edges, duplicate undirected pairs, missing endpoints, invalid masses and invalid lengths;
- position-to-edge consistency validation;
- triangle-inequality validation for closed 3-edge graphs;
- construction of C=G M⁻¹ Gᵀ using the actual body masses;
- deterministic symmetric Jacobi eigenvalue iteration for the 2×2 or 3×3 Gram matrix;
- scale-aware numerical rank and reciprocal condition indicator;
- typed FullRank / IllConditioned / RankDeficient outcomes;
- a separate small-matrix SVD-condition diagnostic for a generally non-symmetric multiplier Jacobian, computed through JᵀJ.

For JᵀJ, the singular-value ratio is sigma_min/sigma_max, not the squared eigenvalue ratio. The implementation squares the policy cutoffs for the eigenvalue test and reports the unsquared singular-value ratio. This avoids mixing a Gram-matrix condition indicator with a Jacobian condition indicator.

The returned GeometryRankReceipt includes graph revision, stable body/edge IDs, numerical rank, smallest/largest Gram eigenvalue, reciprocal condition indicator, per-edge signed length residuals, triangle area and the padded Gram matrix.

## The distinguishing experiment

Use a triangle whose first two sides have length 1 and whose included angle is theta:

\[
q_0=(0,0,0),\quad q_1=(1,0,0),\quad
q_2=(1+\cos\theta,\sin\theta,0).
\]

Then

\[
L_{01}=L_{12}=1,\qquad L_{20}=2\cos(\theta/2),
\]

and its area is \(A=\frac12\sin\theta\).

For every theta > 0 the side lengths satisfy the strict triangle inequality and the three constraint gradients are independent. At theta=0 the three points become collinear, L20=2=L01+L12, and the third constraint becomes dependent: rank falls from 3 to 2.

The default policy uses:
- ill-conditioned reciprocal condition threshold = 1e-6;
- rank-deficient reciprocal condition threshold = 1e-12;
- geometry tolerance = 1e-10.

The rank stress ladder is theta = 0.5, 0.1, 0.05, 0.01, 0.005, 0.001, 1e-4, 1e-5, 0. The report must show monotonically decreasing reciprocal condition as the triangle approaches collinearity. The 1e-4 case is expected to be IllConditioned; the 1e-6 case is expected to be RankDeficient under the default cutoff; the exactly collinear case must be rank 2.

## Important counterexample: an open chain can be collinear and still full-rank

For two links on three bodies, the constraint Jacobian has two independent rows as long as both link vectors are nonzero. Even if the two links are collinear, the two constraints act on different endpoint/middle-body coordinate combinations. The two-edge chain therefore remains rank 2.

This distinction prevents a simplistic “small triangle area means singular” rule from being incorrectly applied to open chains. Rank must be computed from G M⁻¹ Gᵀ, not inferred from a visual angle heuristic.

## Metamorphic and negative controls

Tests cover:
- open-chain full rank for both collinear and non-collinear links;
- regular closed triangle full rank;
- monotonic condition degradation toward collinearity;
- exact collinearity returns rank 2 rather than pretending to have three unique multipliers;
- infeasible triangle lengths rejected before solving;
- input edge permutation preserves deterministic ID ordering and spectrum;
- edge endpoint-orientation reversal preserves geometric spectrum;
- generic SO(3) rotation preserves rank and spectrum;
- separate multiplier-Jacobian SVD conditioning;
- duplicate IDs, self edge and invalid mass failures.

The next graph tranche should add duplicate undirected edges, missing endpoints, inconsistent in-range positions, equal-mass scaling, very large/small mass ratios, and near-rank-threshold fuzz/property tests.

## Independent algebraic expectations

For the frozen mass profile (1.3, 2.1, 0.8 kg), a separate double-precision calculation gives approximate Gram reciprocal-condition indicators:

| theta (rad) | lambda_min(C)/lambda_max(C) |
|---:|---:|
| 0.5 | 1.2366e-2 |
| 0.1 | 4.7677e-4 |
| 0.05 | 1.1906e-4 |
| 0.01 | 4.7605e-6 |
| 0.005 | 1.1901e-6 |
| 0.001 | 4.7604e-8 |
| 1e-4 | 4.7604e-10 |
| 1e-5 | 4.7605e-12 |
| 0 (collinear) | approximately 0 |

These are independent numerical checks of the matrix definition, not Rust CI output. The exact values from the pinned Rust implementation will be captured by the workflow.

## Why this improves on the first open-chain reference

#3144 already supplies a coupled pHDAE open-chain time step, a coupled multiplier solve and an independent continuous constrained-RK4 trajectory reference. It does not claim arbitrary graph topology or a rank-revealing closed-loop solver.

This module adds the missing preflight distinction:
1. the geometric constraint manifold rank;
2. the timestep-dependent discrete multiplier-Jacobian conditioning.

It does not yet change #3144's time-step behavior. The next implementation should make the graph object the source of stable edge identity, call this preflight before a closed-loop multiplier solve, and refuse to emit a unique multiplier vector when the Gram matrix is rank-deficient.

## Industry comparison protocol

The geometric rank problem is a separate numerical qualification from comparing complete physics engines. Current reference candidates include:
- MuJoCo 3.17.0: its changelog documents a discrete integrator and recent constraint-solver changes; its 3.17.0 batch API reports bit-identical results to plain stepping at any thread count. Exact model/joint/solver settings must still be pinned. [Release notes](https://mujoco.readthedocs.io/en/3.17.0/changelog.html)
- Project Chrono 10.0.0: released 27 March 2026; includes multibody/implicit-integrator Jacobian improvements and wider FSI/peridynamics capabilities. [Release announcement](https://projectchrono.org/news/)
- Drake 1.58.0: released 15 October 2026. [Release notes](https://drake.mit.edu/release_notes/release_notes.html)
- Pinocchio 4.1.0: latest release observed 7 July 2026; useful for articulated-body algorithms and constrained dynamics, but not by itself an end-to-end time-stepping/contact engine. [Releases](https://github.com/stack-of-tasks/pinocchio/releases)
- DART 6.20.2: latest release observed 19 October 2026, with recent contact and constraint-related fixes. [Releases](https://github.com/dartsim/dart/releases)
- NVIDIA PhysX SDK 5.9.0: current SDK release candidate observed in the official release list. [Releases](https://github.com/NVIDIA-Omniverse/PhysX/releases)

The exact external comparison must use the canonical [#443 differential runner](https://github.com/Luminous-Dynamics/symtropy/issues/443), after a real RATTLE chain baseline exists. It must not treat the separate 3D friction-disk PR #3145 as a RATTLE baseline; that PR is a tangential Coulomb contact subproblem. No matched open-chain campaign PR #3146 currently exists.

For the eventual engine comparison, pin exact version, model topology, constraint type, stabilization, solver iterations, warm starting, timestep, output cadence, thread/backend mode, build ID and hardware. Compare per-edge residuals, rank/condition, trajectory error, energy/work, momentum maps, failures, p50/p95/p99 step time and memory. Never compare unlike joint semantics.

## Claim ceiling

This is a bounded 3-body / 2- or 3-edge geometric rank diagnostic. It is not yet a graph-general time integrator, RATTLE baseline, closed-loop pHDAE solver, arbitrary-topology solver, contact/impact/friction implementation, production multibody system, SOTA result or physical validation.

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/constraint-graph-rank/constraint_graph_rank.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/constraint-graph-rank/constraint_graph_rank.rs -o /tmp/constraint-graph-rank-tests
/tmp/constraint-graph-rank-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/constraint-graph-rank/constraint_graph_rank.rs -o /tmp/constraint-graph-rank
/tmp/constraint-graph-rank > /tmp/constraint-graph-rank-a.csv
/tmp/constraint-graph-rank > /tmp/constraint-graph-rank-b.csv
cmp /tmp/constraint-graph-rank-a.csv /tmp/constraint-graph-rank-b.csv
~~~
