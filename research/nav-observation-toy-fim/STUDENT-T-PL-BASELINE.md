# Heavy-tailed Student-t protection-level baseline v0.1

This module implements the direct covariance-based baseline described in Section 5.1 of Maharmeh et al. (2026), citing Al Hage et al. (2020).

The paper sweeps degrees of freedom {4,5,6,7,8,9,10,100}. It calculates an overall PL from the maximum eigenvalue of the 2D position covariance and directional PLs by projecting the eigenvectors into the vehicle along-track/cross-track axes.

The implementation preserves the paper's printed directional equation:
PL_i = K_TIR(dof) sqrt((dof-2) max(lambda_1 |V_1,i|, lambda_2 |V_2,i|)).

K_TIR(dof) is an explicit argument. The paper describes it as the inflation multiplier obtained from the 2D multivariate Student-t CDF for the target TIR. Do not replace this with a one-dimensional normal quantile or reuse a constant across degrees of freedom. The full TIR-to-quantile convention must be verified and frozen in the reproduction manifest.

## References

- Maharmeh et al., 2026, https://doi.org/10.3390/s26165140, Section 5.1.
- Al Hage et al., 2020, https://doi.org/10.1109/TITS.2020.2983681.

## Limits

This computes PL from a supplied 2D covariance and supplied 2D Student-t inflation multiplier. It does not estimate the covariance, select/fit degrees of freedom, detect/exclude faults, or validate the distributional assumptions. It is a finite equation-level comparator, not a complete GNSS/INS integrity implementation.
