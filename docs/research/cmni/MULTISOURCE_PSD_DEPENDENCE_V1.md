# CMNI finite multi-source PSD dependence design v1

**Status:** Rust finite-scenario oracle authored; exact-head CI and scientific review pending.  
**Implementation:** `research/cmni/reference-kernel/src/multisource_dependence_design.rs`.  
**Related two-source work:** `dependence_envelope.rs` and `robust_dependence_design.rs`.

## 1. Why a matrix-valued ambiguity set is needed

For two sources, a single correlation rho in [-1,1] fully describes the cross-source dependence. For three or more sources, independently choosing every pairwise correlation from a range can create a matrix that is not positive-semidefinite (PSD), and therefore cannot be a valid joint covariance.

The natural feasible set is a subset of the correlation elliptope:
[
\mathcal R = \{R=R^\top,\; R\succeq0,\; \operatorname{diag}(R)=\mathbf 1,\; L_{ij}\le R_{ij}\le U_{ij}\}.
]

The elliptope constraint couples the entries. This is not just a numerical nuisance: a robust worst-case design should only consider physically/statistically admissible joint source covariance matrices.

Robust optimization has studied correlation ambiguity with PSD-constrained covariance uncertainty sets. For example, the robust portfolio literature formulates correlation uncertainty together with a PSD matrix constraint, instead of treating pairwise correlations as independent intervals: https://link.springer.com/article/10.1007/s00245-022-09856-1. This is an adjacent mathematical foundation, not a direct CMNI nuclear-data comparator.

## 2. Implemented finite-scenario oracle

The Rust implementation takes:
- a shared prior variance for one scalar parameter;
- shared source marginal standard deviations;
- finite candidate designs, each with observation sensitivities, independent diagonal noise, and an observation-by-source Jacobian;
- a finite list of complete correlation matrices.

For each candidate c and supplied matrix R, it assembles:
[
\Sigma_u(R)=D_\sigma R D_\sigma,
\qquad
V_c(R)=D_{\epsilon,c}+J_c\Sigma_u(R)J_c^\top,
]
then computes:
[
\mathcal I_c(R)=h_c^\top V_c(R)^{-1}h_c,
\quad
\operatorname{EIG}_c(R)=\tfrac12\log(1+\sigma_\theta^2\mathcal I_c(R)).
]

The robust rank is the descending order of the minimum EIG over the **finite supplied scenario set**. The nominal independence baseline is shown only when the identity matrix is among those scenarios.

Scenario validation checks:
- exact square dimensions;
- finite values;
- symmetric entries;
- unit diagonal;
- PSD via a small-matrix Jacobi eigenvalue iteration;
- unique, non-empty scenario identifiers.

Singular PSD source matrices are allowed. The independent observation-noise diagonal must be strictly positive, so the observation covariance remains positive-definite and can be Cholesky factored.

The result reports the worst/best scenario IDs, EIG, posterior parameter variance, nominal and robust ranks, and rank reversal. It marks the computation exact only over the finite enumerated scenario set and explicitly sets formal interval arithmetic to false.

## 3. Three-source synthetic rank-reversal fixture

Machine-readable fixture: `research/cmni/fixtures/three-source-psd-rank-reversal-v1.json`.

Three sources share unit marginal standard deviations. Three PSD scenarios are enumerated:
- identity / independent;
- source-1/source-2 correlation -0.75;
- source-1/source-2 correlation +0.75;
- source 3 is independent in each supplied scenario.

The two candidate designs reproduce the two-source negative control with the third source added as an inert source:
- nominal independence prefers `nominal-winner`, EIG 0.4489707966 nats;
- finite-scenario minimax prefers `robust-winner`, worst-case EIG 0.3589198966 versus 0.3404385440 nats;
- nominal choice loses 0.0184813526 nats of worst-case EIG.

The fixture additionally rejects the all-off-diagonal -0.9 3x3 matrix. That matrix satisfies each pairwise coefficient bound [-1,1] but is not PSD, showing why independent pairwise interval handling is wrong.

The oracle also accepts the rank-one all-ones correlation matrix as a valid singular PSD common-mode scenario. The observation covariance remains SPD because independent measurement noise is positive.

## 4. Comparison and limits

- **Two-source analytic envelope:** continuous 1-D interval; global extrema by endpoints plus stationary points, but f64 and not formally enclosed.
- **Multi-source finite scenarios (this module):** any finite source count, exact minimum/maximum only over explicitly supplied PSD matrices; no continuous-set coverage.
- **Full PSD-uncertainty robust design (future):** optimize over all correlation matrices satisfying the PSD and entrywise evidence constraints. The lower scalar information problem has a convex structure because (h^\top V(R)^{-1}h) is convex in positive-definite V and V is affine in R. A small-instance SDP reference can exploit this; a solver's numerical tolerance and primal/dual gap still need independent qualification. Upper extrema and more general QoI/risk objectives may require nonconvex global bounds.

The next serious step is not to silently interpret a hand-picked scenario list as the complete elliptope. Instead, compare finite scenario rankings with a verified small-dimensional continuous PSD optimizer and report the gap.

## 5. Benchmarks and targets

| Suite | Control | Required metric |
|---|---|---|
| PSD validity | known PSD / known indefinite matrices | zero invalid matrices accepted |
| Correlation geometry | pairwise interval corners vs elliptope-feasible matrices | infeasible-corner rejection |
| Linear-Gaussian parity | direct dense covariance solve | EIG and posterior-variance error |
| Rank stability | nominal, finite-scenario minimax, continuous SDP oracle | rank-reversal rate and worst-case regret |
| Source count scaling | 2, 3, 4, 8, 16 sources | runtime, eigensolver iterations, Cholesky cost |
| Singular source modes | rank-deficient PSD matrices with independent noise | stable finite result |
| Missing source | omitted common-mode factor | posterior undercoverage and false confidence |
| Evidence qualification | missing / mismatched scope receipt | zero qualified ranking claims |

Aspirational targets (not achieved):
- zero PSD-invalid matrices accepted on analytic and randomized controls;
- agreement with an independent dense covariance implementation to 1e-10 on well-conditioned cases;
- zero hard failures for singular PSD source covariance when observation noise is strictly positive;
- publish regret against a continuous PSD reference on every tractable problem;
- no continuous-set guarantee claimed from a finite scenario grid;
- no formal numerical certificate claimed until outward-rounded arithmetic and independent verification exist.

## 6. Further research

1. Replace the finite scenario list with interval-constrained correlation matrices in the elliptope.
2. Implement an SDP lower-bound oracle for scalar-parameter minimum information and compare primal/dual bounds.
3. For worst-case and nonlinear QoI objectives, use global branch-and-bound with PSD relaxations rather than a local optimizer presented as a global result.
4. Extend from one scalar parameter to matrix-valued parameter information, then to linear QoI and predictive QoI.
5. Couple the resulting robust utility ranges into the interval-dominance gate and source-calibration EVSI controller.
6. Validate source ranges using independent calibration campaigns and held-out source families.

No physical detector, nuclear-data or experimental validation is established by these synthetic fixtures.
