# Heavy-tailed Student-t protection-level baseline v0.1

This module implements the direct covariance-based baseline described in Section 5.1 of Maharmeh et al. (2026), citing Al Hage et al. (2020).

The paper sweeps degrees of freedom {4,5,6,7,8,9,10,100}. It calculates an overall PL from the maximum eigenvalue of the 2D position covariance and directional PLs by projecting the eigenvectors into the vehicle along-track/cross-track axes.

The implementation preserves the paper's printed directional equation:
PL_i = K_TIR(dof) sqrt((dof-2) max(lambda_1 |V_1,i|, lambda_2 |V_2,i|)).

The implementation now provides `student_t_k_tir_2d(dof, tir)` for the standard 2D radial-tail convention when P_g is the covariance matrix (not the Student-t scale matrix). For a 2D multivariate Student-t, the squared Mahalanobis radius has the scaled F(2,dof) law; with the paper's `(dof-2) * covariance` normalization, the radial tail is `(1 + K_TIR^2)^(-dof/2)`. Solving for the requested total 2D radial tail probability gives `K_TIR = sqrt(tir^(-2/dof) - 1)`. The helper and convenience API are tested for dof {4,5,6,7,8,9,10,100}. The caller must still verify that the paper's TIR is interpreted as this total radial tail (rather than per-axis/two-sided marginal coverage) before comparing route metrics. Never substitute a 1D normal quantile.

## Directional projection audit

The implementation preserves the article's printed directional equation verbatim for reproduction. A separate `audit_directional_projection` function compares it with the conventional covariance projection:

- printed form: max(lambda_j * |v_{j,i}|);
- covariance projection: e_i^T P e_i = sum_j lambda_j * v_{j,i}^2.

These expressions are not generally equal. A deterministic 45-degree example with eigenvalues 2 and 1 gives covariance-projected variance 1.5, while the printed max term is sqrt(2) ≈ 1.4142. With the same degrees of freedom and radial multiplier, the printed directional PL is approximately 2.90% lower in that example. This is a mathematical counterexample, not a claim about the paper's reported route results. The baseline comparison must preserve the printed equation and report the covariance-projected alternative separately pending author/source-code clarification.

The radial multiplier also depends on `TIR` meaning a total two-dimensional radial tail probability and on `P_g` being the covariance matrix rather than the Student-t scale matrix. Per-axis marginal coverage is a different convention. Do not fit or select degrees of freedom on held-out test routes.

## References

- Maharmeh et al., 2026, https://doi.org/10.3390/s26165140, Section 5.1.
- Al Hage et al., 2020, https://doi.org/10.1109/TITS.2020.2983681.

## Limits

This computes PL from a supplied 2D covariance and supplied 2D Student-t inflation multiplier. It does not estimate the covariance, select/fit degrees of freedom, detect/exclude faults, or validate the distributional assumptions. It is a finite equation-level comparator, not a complete GNSS/INS integrity implementation.
