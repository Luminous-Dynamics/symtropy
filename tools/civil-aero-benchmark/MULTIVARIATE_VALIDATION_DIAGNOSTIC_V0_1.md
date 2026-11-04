# AERO Multivariate Validation Diagnostic v0.1

## Purpose and boundary

This module compares a vector of predicted engineering outputs with the corresponding blind-confirmation observations while retaining the declared cross-output correlation structure. It is intended for coupled quantities such as lift, drag and pitching moment, or pressure at multiple spatial stations.

It is a **joint discrepancy diagnostic**, not a CFD solver, uncertainty estimator, calibration algorithm, validation pass/fail gate, or source authenticator. The uncertainty source digest binds the caller's declared source bytes by identity; this module does not independently prove that the source is a valid experimental uncertainty budget.

The design is informed by ASME VVUQ 20.1, *Multivariate Metric for Validation*. That standard describes a global discrepancy assessment across multiple variables, locations and/or time instants, including shared uncertainty. This module is a narrow first executable contract and must not be described as a conforming implementation of the full standard.

Reference: https://www.asme.org/codes-standards/vvuq-standards

## Metric

For output component i:

`z_i = (y_{pred,i} - y_{obs,i}) / u_i`

where `u_i` is the declared standard uncertainty of the prediction-minus-observation discrepancy for output i.

Let `R` be the caller-supplied correlation matrix in exactly the same output order. It must be dimensionless, symmetric, unit diagonal and strictly positive definite. Compute its Cholesky factor `R = L L^T`, then solve:

`L w = z`

The module reports:

`d^2 = w^T w = z^T R^{-1} z`

It never constructs an explicit matrix inverse. The output includes both the marginal standardized residuals `z` and whitened components `w`, so users can inspect which dimensions contribute to the joint discrepancy.

## Critical interpretation limits

- `d^2` is not automatically a chi-square statistic. No chi-square degrees of freedom, p-value, probability of validity, interval coverage, or universal acceptance threshold is emitted.
- A positive-definite matrix supplied by a caller is not proof that its uncertainty model is physically adequate or calibrated.
- The uncertainty source is scoped to each observation with a reference, SHA-256 and rationale. The digest proves only byte identity if the supplied bytes are actually checked by a separate provenance process.
- Every scored observation must use `blind_confirmation`. Calibration/training rows are rejected.
- One study fixes the output names, units and order. Missing, reordered or unit-shifted outputs are errors, not silently aligned.
- In-domain and out-of-domain observations are retained and counted separately. OOD scores are diagnostic; they do not enlarge the applicability domain.
- Synthetic unit tests verify software arithmetic and negative controls only. They are not physical validation evidence.

## Independent interchange

`joint_validation_report_to_csv` emits one row per output, with repeated identities, the correlation row, standardized and whitened residuals, per-observation digest, study digest and report digest. The stdlib Python oracle independently reconstructs the correlation matrix, runs Cholesky and forward substitution, and replays all digests and summary statistics.

## Planned qualification ladder

1. Known-answer synthetic fixtures for identity and correlated matrices.
2. Negative controls for non-blind data, reordered axes, mismatched units, singular/indefinite/asymmetric matrices, duplicate observations and tampered source identities.
3. Blind multi-output validation against public NASA TMR references (for example NACA0012 pressure distributions) and DPW-8 OAT15A data, only after exact source bytes, licenses, geometry permissions, measurement uncertainty and output definitions are admitted.
4. Add uncertainty correlation evidence from shared experimental calibration, common tunnel conditions, shared geometry/mesh inputs and common model-form terms.
5. Only after independent calibration and distributional validation, consider thresholds or probabilistic interpretations. Any such gate must be a separate, versioned policy.

Public reference ecology:
- ASME VVUQ standards: https://www.asme.org/codes-standards/vvuq-standards
- NASA Turbulence Modeling Resource: https://tmbwg.github.io/turbmodels/
- NASA TMR reference data listings: https://www.nasa.gov/reference/turbulence-modeling-resource-large-files/
- AIAA DPW-8 cases: https://www.aiaa-dpw.org/TestCases/testcases.html
