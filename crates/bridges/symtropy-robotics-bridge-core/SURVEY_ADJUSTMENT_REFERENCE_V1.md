# Survey Adjustment Reference Corpus v1

**Status:** deterministic synthetic algebraic reference cases. These are not field observations and have not yet been cross-compared with NOAA NGS ADJUST, LOCUS, or another independent geodetic package.

This corpus specifies small, independently hand-computable cases for the linearized weighted least-squares contract implemented in `src/survey_adjustment.rs`. It is meant to detect regressions in weighting, datum conditioning, column-unit scaling, residual signs, degrees of freedom, and formal covariance.

## Conventions

For each fit observation:

- Model: `coefficients · parameters ≈ observed_value`.
- Standard uncertainty σ is one standard deviation; weight is (1/\sigma^2).
- Reported residual is **predicted minus observed**.
- Degrees of freedom are (n-r), where (n) is the number of fitting observations and (r) the number of free parameters.
- Formal covariance is ((A^T P A)^{-1}) over free parameters, where (P=\operatorname{diag}(1/\sigma_i^2)). The posterior variance factor is reported separately and does not silently rescale this matrix.
- Fixed parameters are treated as exact in these cases; their uncertainty is not propagated.

## Case A — Unequal-weight two-parameter solution

Unknowns (x,y). Observations:

| ID | Equation coefficients ([x,y]) | Observed | σ |
|---|---:|---:|---:|
| east | ([1,0]) | 10 | 1 |
| north | ([0,1]) | 20 | 2 |
| diagonal | ([1,1]) | 31 | 1 |

The normal matrix and right-hand side, shown here as a reference calculation (the production solver uses SVD of the weighted design, not normal equations), are:

[
A^T P A = \begin{bmatrix}2&1\\1&1.25\end{bmatrix},\qquad
A^T P l = \begin{bmatrix}41\\36\end{bmatrix}.
]

Expected estimates and formal covariance:

[
\hat{x}=\frac{61}{6},\qquad \hat{y}=\frac{62}{3}
]

[
C_{xy}=\begin{bmatrix}5/6&-2/3\\-2/3&4/3\end{bmatrix}.
]

The fit residuals in input order are (1/6, 2/3, -1/6). Weighted residual sum of squares is (1/6), with one degree of freedom and posterior variance factor (1/6).

**Test:** `two_parameter_unequal_weight_solution_matches_hand_computed_reference`.

## Case B — Same fit under a severe parameter-unit rescaling

Use the same values and uncertainties as Case A, but multiply the second column of every design row by (c=10^{-12}). The fitted second parameter is expressed in the reciprocal scaled unit, so its estimate and covariance must transform:

[
\hat{y}_{scaled}=\frac{1}{c}\frac{62}{3},\qquad
C_{scaled}=\begin{bmatrix}
5/6&-\frac{2}{3c}\\
-\frac{2}{3c}&\frac{4}{3c^2}
\end{bmatrix}.
]

The relative rank tolerance in this regression is (10^{-10}). The physical fit is unchanged, but the raw weighted design columns differ by twelve orders of magnitude. A raw-scale singular-value cutoff could incorrectly reject the second column. The solver normalizes each nonzero weighted column by its Euclidean norm (using stable `hypot` accumulation), applies SVD to the equilibrated matrix, and transforms estimates and covariance back to the supplied parameter units.

Column equilibration reduces sensitivity to arbitrary numeric units; it does **not** repair highly correlated columns, poor network geometry, or a genuinely unobservable parameter.

**Test:** `column_equilibration_preserves_solution_and_covariance_across_parameter_units`.

## Case C — Constrained three-leg leveling loop and misclosure

Anchor point A at (h_A=0). Unknown heights are (h_B,h_C); fit observations are:

| ID | Equation | Observed (m) | σ (m) |
|---|---|---:|---:|
| loop-ab | (h_B-h_A) | 10.02 | 0.01 |
| loop-bc | (h_C-h_B) | 4.99 | 0.02 |
| loop-ca | (h_A-h_C) | -15.04 | 0.01 |

The observed loop closure is (-0.03\,m). With the declared uncertainties, the independent reference distributes the (+0.03\,m) residual in proportion to the observation variances (1:4:1). Thus the predicted-minus-observed residuals are (+0.005,+0.020,+0.005\,m) and their sum cancels the observed misclosure.

Expected solution:

[
\hat h_A=0,\qquad \hat h_B=10.025\,m,\qquad \hat h_C=15.035\,m.
]

The reduced design has rows ([1,0],[-1,1],[0,-1]), with weights (10000,2500,10000). In the ([h_B,h_C]) order:

[
A^T P A=\begin{bmatrix}12500&-2500\\-2500&12500\end{bmatrix},\qquad
A^T P l=\begin{bmatrix}87725\\162875\end{bmatrix}.
]

Expected formal covariance:

[
C_h=\begin{bmatrix}
1/12000&1/60000\\
1/60000&1/12000
\end{bmatrix}\ m^2.
]

Expected weighted residual sum of squares is (1.5); degrees of freedom are (1), so the posterior variance factor is (1.5).

This is intentionally a **single loop with an exact datum anchor**. It tests loop misclosure distribution and redundant-observation diagnostics. It does not model random anchor uncertainty, correlated observations, gross-error rejection, instrument calibration, Earth curvature, height-datum transformations, or nonlinear field-observation equations.

**Test:** `weighted_loop_adjustment_distributes_misclosure_by_declared_variance`.

## External validation gate

The next reference-corpus increment should add pinned comparisons against reference software, recording exact software version, input hashes, configuration hashes, constraints, units, result hashes, and documented tolerances. Keep free/minimally constrained and constrained solutions as separate cases; preserve independent checkpoints and negative controls.

Relevant primary references:

- [LAPACK DGESVX](https://www.netlib.org/lapack/explore-html/d5/dbe/group__gesvx_ga82173a93234afc15d70b64233b3e5bc8.html) documents matrix equilibration, solution back-transformation, and condition/error estimates.
- [NOAA NGS ADJUST](https://geodesy.noaa.gov/PC_PROD/ADJUST/) adjusts horizontal, vertical-angle, and GPS observations.
- [NOAA NGS LOCUS](https://ngs.noaa.gov/locus/) distinguishes free/minimally constrained and constrained leveling adjustments and documents loop redundancy and misclosure distribution.
- [NGS OPUS Projects network-adjustment sequence](https://beta.ngs.noaa.gov/OPUS-Projects/user_guide/section12.1.html) uses successive preliminary, free, and constrained adjustment stages.
- [NIST AMS 100-85](https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing) discusses risk-based model credibility and VVUQ. Engineering decisions with greater consequence require stronger independent evidence than synthetic algebraic tests.

These sources motivate test design; **the expected values in this file were derived algebraically and are not represented as outputs from the external software**.

## Reproduction

From the repository workspace:

```sh
cargo test --locked -p symtropy-robotics-bridge-core survey_adjustment::tests::
```

The project qualification workflow additionally checks workspace formatting and Clippy with warnings denied. Report validation only against the exact commit SHA tested by the workflow.
