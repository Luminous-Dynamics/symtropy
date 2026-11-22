# CMNI primal–dual certificate verification for continuous dependence design v1

**Status:** independent f64 verifier authored; exact-head CI and review pending.  
**Implementation:** `continuous_psd_sdp_certificate.rs`.  
**Prerequisites:** scalar-parameter multi-source PSD dependence model and continuous pairwise-interval envelope.

## 1. Continuous robust-information problem as an SDP

For observations
[
y=h\theta+Ju+\epsilon,
\qquad
V(R)=D_\epsilon+J D_\sigma R D_\sigma J^T,
]
the scalar parameter information is
[
f(R)=h^T V(R)^{-1}h.
]

The continuous worst-case information is the solution to
[
\begin{aligned}
\min_{R,t}\quad &t\\
\text{s.t.}\quad&
\begin{bmatrix}
V(R)&h\\h^T&t
\end{bmatrix}\succeq0,\\
&R\succeq0,\quad \operatorname{diag}(R)=\mathbf1,\\
&l_{ij}\le R_{ij}\le u_{ij}.
\end{aligned}
]
Because independent observation noise is positive, (V(R)) is positive definite for every admissible source-correlation matrix. By the Schur complement, the block LMI is equivalent to (t\ge h^T V(R)^{-1}h).

This is an SDP in the scalar-parameter case: the observation LMI is affine in R and t; the source correlation matrix is constrained to the elliptope with pairwise evidence bounds.

## 2. Dual formulation checked by the verifier

Let X be the PSD multiplier for the observation LMI, Y the PSD multiplier for R, (alpha_i) the free equality multipliers for (R_{ii}=1), and (lambda^L_{ij},lambda^U_{ij}\ge0) the pairwise lower/upper interval multipliers.

The verifier checks stationarity:
[
1-X_{n+1,n+1}=0,
]
[
-\langle X,A_{ii}\rangle-Y_{ii}+\alpha_i=0,
]
[
-\langle X,A_{ij}\rangle-2Y_{ij}-\lambda^L_{ij}+\lambda^U_{ij}=0.
]
Here
[
A_{ii}=\begin{bmatrix}
\sigma_i^2 j_i j_i^T&0\\0&0
\end{bmatrix},
\quad
A_{ij}=\begin{bmatrix}
\sigma_i\sigma_j(j_i j_j^T+j_j j_i^T)&0\\0&0
\end{bmatrix}
quad(i<j),
]
where (j_i) is column i of J.

The dual objective checked is
[
d=-\langle X,M_0\rangle
+\sum_{i<j}(\lambda^L_{ij}l_{ij}-\lambda^U_{ij}u_{ij})
-\sum_i\alpha_i,
\qquad
M_0=\begin{bmatrix}D_\epsilon&h\\h^T&0\end{bmatrix}.
]

A primal-feasible epigraph t is an upper bound on the minimum. A dual-feasible objective d is a lower bound. Their gap brackets the continuous optimum when both witnesses satisfy the constraints.

## 3. Implemented verification

`verify_continuous_dependence_sdp_certificate` takes:
- the candidate linear-Gaussian model and pairwise source-correlation bounds;
- a primal feasible correlation matrix and epigraph value t;
- observation-LMI dual matrix X;
- source-correlation dual matrix Y;
- one pair of nonnegative lower/upper multipliers for every i<j;
- free diagonal equality multipliers;
- a non-empty evidence-scope receipt reference;
- a caller-specified feasibility tolerance.

It independently checks:
1. candidate model dimensions and positivity constraints;
2. primal R symmetry, unit diagonal, PSD and pairwise bounds;
3. observation block-LMI PSD;
4. X and Y symmetry and PSD;
5. complete unique coverage of pairwise multipliers, non-negativity, and diagonal multiplier shape;
6. t and every diagonal/off-diagonal stationarity equation;
7. dual objective, primal objective, and the primal–dual gap;
8. conversion of the information bracket to a worst-case EIG bracket.

Invalid witnesses return an error rather than a best-effort “certificate”.

The evidence receipt is stored in the certificate result, but the verifier does not authenticate the receipt or check that the supplied physical correlation bounds are scientifically justified.

## 4. Exact synthetic zero-gap benchmark

Fixture: `research/cmni/fixtures/two-source-sdp-primal-dual-certificate-v1.json`.

One observation has h=1, independent noise variance 1, two unit source scales and unit source Jacobians. The correlation range is [-0.5,0.5]. The worst case is rho=+0.5:
- (V=4);
- primal t = 0.25;
- (X=\begin{bmatrix}0.0625&-0.25\\-0.25&1\end{bmatrix}\);
- (Y=0\);
- (alpha=[0.0625,0.0625]);
- (lambda^L_{12}=0), (lambda^U_{12}=0.125);
- dual objective = 0.25;
- primal–dual gap = 0;
- worst-case EIG = 0.1115717756571 nats.

The unit test also corrupts the dual multiplier, makes X non-PSD, puts the primal R outside the admitted interval, removes a pairwise multiplier, and makes the epigraph too small. Each invalid case is rejected.

## 5. Numerical and epistemic boundary

The implementation uses f64 and a Jacobi eigenvalue routine. It checks PSD to a caller-specified scale-aware tolerance and stationarity/gap to a scale-aware tolerance. It does not use directed rounding, exact arithmetic, interval Cholesky, or an independent formally verified checker.

Therefore the status is `VerifiedWithinFloatingPointTolerance`, and `formal_certificate` is always false. The result is a machine-checkable, reviewable numerical optimization receipt, not a formal mathematical proof. It is stronger than trusting a solver-reported objective value but does not eliminate floating-point verification risks.

## 6. Industry/research comparison

The verifier is intended to support a reproducible continuous-ambiguity benchmark:
- compare the convex-tangent lower/feasible-witness upper bracket with a solver-generated SDP primal/dual pair;
- verify the pair independently in this Rust kernel;
- use the SDP primal matrix as a new tangent anchor;
- compare gaps, runtime, conditioning and source count;
- only after numerical verification, route bounds into the shared interval-dominance gate.

The 2026 maximin robust Bayesian OED work of Abdulsamad et al. addresses broader model ambiguity using Sibson alpha-mutual information and PAC-Bayes lower bounds: https://arxiv.org/abs/2603.14094. CMNI's SDP is a narrower, physically interpretable covariance ambiguity model rather than a replacement for their distributional uncertainty set.

The constrained sequential BED work COPEx combines amortized posterior/policies and online scenario-tree lookahead: https://proceedings.mlr.press/v306/guo26g.html. The SDP verifier complements that planner by allowing a continuous dependence ambiguity set to supply checked lower/upper design utility bounds. It does not implement a campaign scheduler.

## 7. Benchmark targets (not achieved)

- exact agreement with analytic one-dimensional robust problems;
- zero accepted invalid primal/dual witnesses in negative controls;
- stationarity residual below declared tolerance;
- dual objective never materially above primal epigraph;
- report primal-dual gap and every PSD/stationarity residual;
- comparison with at least one independent SDP solver on 2–8 sources;
- no formal certification claim until an independent outward-rounded or exact verifier is available.

No physical data validation or SOTA superiority is established by these synthetic tests.
