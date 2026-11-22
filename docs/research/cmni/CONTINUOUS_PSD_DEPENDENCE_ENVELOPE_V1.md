# CMNI continuous PSD dependence envelope v1

**Status:** convex-tangent lower/feasible-witness upper bracketing oracle authored; exact-head CI and independent numerical review pending.  
**Implementation:** `research/cmni/reference-kernel/src/continuous_dependence_envelope.rs`.  
**Parent implementation:** finite PSD scenario validation and scalar-parameter information in `multisource_dependence_design.rs`.

## 1. Why this is different from a finite scenario grid

The finite multi-source design oracle is exact only over the matrices explicitly supplied. A grid of PSD-feasible correlation matrices does not establish a minimum over the continuous elliptope. CMNI needs a way to bracket the continuous worst case even before it has a full SDP solver.

For a scalar parameter with observation model
[
y = h\theta + Ju + \epsilon,
\qquad
V(R)=D_\epsilon+J D_\sigma R D_\sigma J^T,
]
the parameter information is
[
f(R)=h^T V(R)^{-1}h.
]

The matrix-fractional function (h^T V^{-1}h) is convex in positive-definite (V). At a feasible reference matrix (R_0), let
[
x_0=V(R_0)^{-1}h,
\qquad
b=D_\sigma J^T x_0.
]
The global supporting-hyperplane inequality gives
[
f(R)\ge f(R_0)-b^T(R-R_0)b
=f(R_0)+b^T R_0 b-b^T Rb.
]

For fixed b and pairwise correlation intervals (l_{ij}\le R_{ij}\le u_{ij}), the maximum of the quadratic term over the *relaxed set that ignores PSD coupling* is explicit:
[
\max b^T Rb
=
\sum_i b_i^2
+2\sum_{i<j}\max(b_i b_j l_{ij}, b_i b_j u_{ij}).
]
Since the actual PSD-constrained set is a subset of this relaxed set, substituting this relaxed maximum gives a valid lower bound on f(R) for every feasible correlation matrix.

A feasible PSD witness (R_w) gives (f(R_w)\ge\min_R f(R)), hence an upper bound on the unknown continuous minimum. Multiple witnesses produce multiple tangents; the maximum of their lower bounds remains a lower bound. The minimum information among the witnesses remains an upper bound.

Therefore the routine returns a bracket:
[
L\le\min_{R\in\mathcal R}h^T V(R)^{-1}h\le U.
]
Mapping this bracket through the monotonic Gaussian formulas gives a bracket on worst-case EIG and posterior variance.

## 2. Implemented contract

Inputs:
- a scalar-parameter candidate design with any positive observation count and source count;
- one symmetric pairwise correlation interval for every off-diagonal source pair;
- at least one complete feasible PSD correlation witness;
- a dependence-scope receipt reference for the source definitions and interval assumptions.

For each witness, the implementation:
1. validates finite values, symmetry, unit diagonal and PSD via the shared Jacobi eigenvalue routine;
2. checks the witness lies inside every pairwise interval;
3. assembles the full observation covariance;
4. computes (f(R_0)) and the source-coordinate gradient b;
5. computes the supporting-hyperplane lower bound using the pairwise interval relaxation;
6. subtracts a declared numerical safety margin;
7. combines all tangent lower bounds by taking their maximum.

The upper bound is the smallest information value among supplied feasible witnesses.

The result reports the information, EIG and posterior-variance brackets, the witness used for the upper bound, tangent-anchor IDs, and the numerical tolerance metadata. The routine fails closed for an empty witness set, duplicate witness IDs, non-PSD matrices, witnesses outside the declared intervals, and invalid interval matrices.

## 3. Exact arithmetic versus implementation status

The convexity argument and the pairwise-relaxation inequality are mathematical statements over exact real arithmetic. The Rust implementation uses f64 Cholesky solves, f64 Jacobi PSD screening, f64 quadratic forms, and a small explicit safety margin.

Consequently:
- the returned numbers are an exploratory numerical bracket;
- `formal_interval_arithmetic` is always false;
- no outward-rounded arithmetic or independently verified floating-point error bound exists;
- the dependence receipt is a reference string, not authenticated evidence;
- a witness proves non-emptiness only for the supplied numerical model and its declared correlation intervals;
- the lower relaxation may be conservative because it discards PSD coupling when maximizing the tangent.

This is stronger than merely reporting the best observed value on a finite grid, but it is not a formally certified mathematical enclosure yet.

## 4. Frozen one-observation example

Fixture: `research/cmni/fixtures/two-source-continuous-worst-case-information-bracket-v1.json`.

One observation has h=1, independent noise variance 1, two unit-scale sources with unit Jacobian, and rho in [-0.5,0.5]. Then
[
V(\rho)=3+2\rho,
\qquad
f(\rho)=\frac{1}{3+2\rho}.
]
The exact worst case is at rho=+0.5:
- minimum information: 0.25;
- worst-case EIG: 0.1115717756571 nats;
- worst-case posterior variance: 0.8.

Including the rho=+0.5 witness as an anchor makes the supporting-hyperplane lower bound tight for this monotone one-dimensional example; the feasible witness supplies the matching upper bound. The unit test pins the resulting bracket.

A second fixture sweeps 10,001 correlations for a two-observation model and checks every direct information value is within the returned bracket. This dense sweep is a regression control; it is not itself the proof of the continuous bound.

## 5. SOTA and algorithmic next steps

The immediate comparator is continuous robust optimization over the correlation elliptope, not another finite scenario grid. Robust portfolio optimization under correlation uncertainty likewise treats the correlation matrix as a joint PSD object rather than independent pairwise coefficients:
https://link.springer.com/article/10.1007/s00245-022-09856-1

The natural next reference is a semidefinite-program formulation. Introduce scalar t and impose
[
\begin{bmatrix}
V(R)&h\\h^T&t
\end{bmatrix}\succeq0,
\quad R\succeq0,
\quad \operatorname{diag}(R)=\mathbf1,
\quad l_{ij}\le R_{ij}\le u_{ij},
]
then minimize t. The Schur complement gives (t\ge h^T V(R)^{-1}h). A small independent SDP implementation can serve as a comparison oracle for the convex-tangent envelope. Primal/dual feasibility and gaps must be independently checked before calling such a result certified.

Potential improvements:
1. initialize with feasible witnesses from identity, rank-one common modes, and boundary PSD constructions;
2. generate new witnesses using a verified SDP or constrained optimizer;
3. add each feasible witness as a tangent anchor, monotonically tightening the lower bound;
4. use SDP dual/primal gaps to independently verify the continuous optimum;
5. move from scalar parameters to multivariate parameter and linear-QoI information;
6. couple robust EIG bounds into interval dominance and value-of-information decisions.

The current code implements only the bracketing layer, not the SDP solver or continuous witness generator.

## 6. Benchmark targets (not achieved)

| Benchmark | Required control |
|---|---|
| Analytic one-source / two-source | closed-form worst case |
| Dense two-source sweep | every sampled point contained in bracket |
| 3–8 source elliptope | independent SDP primal/dual reference |
| Pairwise-vs-PSD relaxation | quantify conservatism and impossible pairwise corners |
| Anchor enrichment | lower bound never decreases when a feasible witness is added |
| Witness invalidity | no non-PSD or out-of-range witness accepted |
| Candidate design ranking | lower bound of selected design exceeds upper bounds of alternatives before certified choice |
| Numerical sensitivity | interval widths under conditioning, source scales and safety margins |

Proposed acceptance targets:
- zero dense-grid containment violations on frozen analytic controls;
- lower bound monotonic under witness enrichment;
- zero PSD-invalid witnesses accepted under independent matrix controls;
- agreement with an independent SDP oracle within published primal/dual gaps on tractable cases;
- no claim of formal certification until outward-rounded arithmetic and independent verification exist;
- publish the bound gap, solver gap, runtime, source count, observation count and conditioning diagnostics separately.

No physical nuclear-data validation or SOTA superiority is established by this synthetic numerical work.


## 7. Qualification bridge to experiment selection

The public adapter `continuous_dependence_eig_utility_interval` converts the continuous worst-case EIG bracket into the common interval-dominance input type. It deliberately assigns `UtilityBoundEvidence::Heuristic`, because the current implementation uses f64 arithmetic and has no directed-rounding proof.

This means the interval can be used for exploration and prioritization, but `certify_maximum_utility` returns an unresolved decision rather than a certified winner. The interval-dominance gate will only accept exact, theorem-qualified, or simultaneous familywise anytime-valid evidence. A mathematically motivated bound is not automatically a numerically certified interval.
