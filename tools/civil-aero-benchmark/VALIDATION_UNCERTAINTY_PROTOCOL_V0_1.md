# CFD validation uncertainty budget protocol v0.1

## Purpose

This protocol connects a numerically admitted grid-convergence study to a comparison against experimental data. It separates numerical epistemic uncertainty from input and experimental uncertainty, propagates correlated inputs, and refuses to infer independence when correlation information is absent.

The module accepts only a CfdEvidenceAdmissionReport with the exact numerical-admission status and binds the predicted value to the fine-grid value in the final (finest) GCI triplet. The measured value, input uncertainty sources and experimental uncertainty remain caller-supplied and unverified. This is not independent experimental validation, a type-certification claim, or flight release.

## Numerical uncertainty

The fine-grid GCI is treated as an expanded estimate and converted to a standard uncertainty by a caller-selected factor:
- Gaussian about the fine-grid solution: k = 2.0;
- roughly shifted Gaussian: k in [1.10, 1.15];
- custom: factor and rationale explicitly supplied.

Then:
- u_h = GCI_fine / k_GCI;
- u_num = u_h + u_iterative + u_roundoff.

The numerical terms are added linearly because their dependence is not assumed negligible or independent. The caller provides iterative and round-off standard uncertainty estimates. Missing values are not silently changed to zero.

A caller-selected numerical expansion factor is applied to u_num, but the result is not automatically a joint 95% interval.

## Input uncertainty

For input parameters with local sensitivities J_i = dS/dX_i, standard input uncertainties u_i, and a correlation matrix R, the linearized input variance is:

u_input^2 = (J u)^T R (J u).

The correlation matrix must be finite, symmetric, have unit diagonal, and be positive semidefinite. Positive-semidefiniteness is checked by a Jacobi eigenvalue calculation within the Rust implementation. This module uses local sensitivity propagation only. Strongly nonlinear input response requires a global method such as Monte Carlo or Latin-hypercube sampling; the local result must not be presented as a global uncertainty estimate.

## Experimental and input correlation

The aggregate experimental standard uncertainty is supplied separately and must retain a source reference to its measurement uncertainty budget or calibration record and a method description that identifies the included repeatability, calibration, data-reduction and other material terms. A bare scalar without its source and estimation method is rejected. Only when the caller explicitly declares both groups aleatory, supplies a rationale, and provides correlation rho between input-induced prediction error and experimental error in E = S - D, then:

u_aleatory^2 = u_input^2 + u_exp^2 - 2 rho u_input u_exp.

When the aleatory declaration is absent or rho is absent, the module reports the separate input and experimental components but does not combine them and does not emit a discrepancy interval. A rho value alone is insufficient to establish aleatory behaviour. It does not silently interpret missing correlation as zero.

## Conservative discrepancy envelope

The module separately expands the numerical and aleatory components using caller-specified factors and adds those expanded bounds linearly:

B = U_num + U_aleatory.

It reports the interval [E - B, E + B] as a conservative linear envelope. This is deliberately not called a joint 95% confidence or coverage interval: coverage of the total cannot be inferred by adding separately expanded components without a joint probability model or risk allocation. The output evidence label preserves this ceiling.

## Synthetic known-answer and negative controls

The synthetic admission fixture has three grids with phi(h) = 2 + 0.8 h^2 at h = [4,2,1], structured GCI safety factor 1.25, observed order 2, and exact fine-grid GCI absolute 1.0. It uses:
- u_GCI = 1.0 / 2 = 0.5;
- iterative uncertainty 0.02 and round-off uncertainty 0.005, yielding u_num = 0.525;
- input sensitivity contributions [0.2, -0.3] with correlation 0.4;
- experimental standard uncertainty 0.15 from synthetic-experiment-budget-v0.1 using a declared repeatability/calibration aggregate method, plus explicitly supplied input/experiment error correlation 0.25.

The Rust tests and independent Python oracle verify covariance propagation, numerical linear summation, signed comparison error, conservative envelope, input source/matrix retention, experimental uncertainty source/method lineage, and the no-joint-coverage label. Negative controls reject non-positive-semidefinite or asymmetric correlation matrices, mismatched predicted values, non-admitted runs, incorrect GCI coverage factors and quantity/case mismatches. Missing input/experiment correlation leaves the combined aleatory uncertainty and discrepancy envelope absent.

## Reference

- NIST IR 8298, A Summary of Industrial Verification, Validation, and Uncertainty Quantification Procedures in Computational Fluid Dynamics, especially equations 10 and 14–18 and the discussion of correlation, epistemic numerical uncertainty and GCI: https://nvlpubs.nist.gov/nistpubs/ir/2020/NIST.IR.8298.pdf
- ASME procedure by Celik et al., Procedure for estimation and reporting of uncertainty due to discretization in CFD applications, Journal of Fluids Engineering (2008), DOI: 10.1115/1.2960953.
