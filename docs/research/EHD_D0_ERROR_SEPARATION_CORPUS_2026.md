# EHD-D0-001 — manufactured error-source separation and conservation firewall

**Tracking:** EHD-DISC-001 / #6681  
**Specification:** #6682  
**Status:** reference kernel and adversarial Rust tests authored; local Rust execution unavailable; exact-head CI pending  
**Implementation:** tools/ehd-reference-audit/src/discrepancy.rs

## What changed

The D0 work package needs an executable reference that distinguishes a known residual source from an attractive but incorrect calibration explanation. This increment implements a deliberately bounded eight-component manufactured oracle.

It separates four systematic residual signatures:
- physical-parameter perturbation;
- numerical bias;
- measurement artifact;
- model-form discrepancy.

An unexplained residual is measured separately and must remain inside a supplied measurement-noise L2 bound. The validation-envelope result and conservation-ledger result are also independent inputs: neither is inferred from how well the residual can be decomposed.

## Manufactured decomposition

The residual is formed at a declared observable boundary:

\[
r = y_{\mathrm{observed}} - G_M(\theta_0,d).
\]

Each signature is an eight-component Hadamard row. The four systematic signatures are mutually orthogonal. For source signature \(s_j\), its least-squares coefficient is

\[
\hat a_j = \frac{s_j^\mathsf{T}r}{s_j^\mathsf{T}s_j}.
\]

The reconstructed systematic residual is

\[
\hat r_{\mathrm{sys}}=\sum_j \hat a_j s_j,
\qquad
r_{\mathrm{unexplained}}=r-\hat r_{\mathrm{sys}}.
\]

If two source signatures are correlated beyond the frozen tolerance, the kernel rejects the decomposition and abstains. It does not regularize a confounded attribution into a confident answer.

The remaining residual norm must be no greater than the caller-supplied measurement-noise bound. A real noise covariance estimator is not implemented: the reference uses one deterministic noise direction orthogonal to the systematic basis.

## Conservation firewall and precedence

The independent ledger gate is passed into the oracle as a boolean produced elsewhere. The residual-fitting operation cannot edit it. A deliberately large discrepancy coefficient with a failed momentum/energy ledger still returns **PhysicalLedgerFailure**.

Disposition precedence is:
1. failed independent conservation ledger → PhysicalLedgerFailure;
2. unsupported validation-envelope input → OutOfEnvelope;
3. unexplained residual above measurement-noise bound → NotEvaluable;
4. detected parameter/numerical error → ParameterOrNumericalErrorDetected;
5. detected measurement artifact → MeasurementArtifactUnresolved;
6. detected model-form discrepancy → ModelFormDiscrepancyDetected;
7. otherwise → NoSystematicSourceDetected.

This is an attribution oracle, not permission to correct a force, source term, momentum flux or energy term.

## Frozen manufactured signatures and outcomes

The source rows are intentionally artificial. Their purpose is to make expected coefficients exactly inspectable. The mixed-error case injects parameter coefficient 1.5, numerical bias -0.75, measurement artifact 0.5, model discrepancy 2.0 and a small noise remainder. It recovers all four coefficients within the analytic tolerance.

Other cases cover:
- noise-only residual inside the declared bound;
- duplicate parameter/numerical signatures, which must abstain;
- a fitted model-discrepancy coefficient with a failed independent ledger;
- an out-of-envelope case with a small residual;
- an unexplained residual that exceeds the noise bound.

No vector is derived from Wang et al. or any real thruster dataset.

## Claim ceiling

The kernel does not implement:
- a general discrepancy function or Bayesian calibration;
- correlated residual covariance or a full QR/SVD source decomposition;
- physical EHD parameter estimation or model selection;
- a multidimensional OOD detector;
- profile likelihood, calibrated predictive intervals or empirical coverage;
- physical validation or a solver.

The code-level implementation error class remains covered by software verification, not inferred from a residual signature.

## Next research increments

1. Replace the orthogonal-only basis with rank-revealing QR/SVD and return a condition-aware identifiability report for correlated but separable sources.
2. Add a validated covariance input with factorization provenance; compare conservative bounds and covariance-aware propagation under matched cohorts.
3. Add a parameter/discrepancy confounding corpus with profile likelihood and explicit parameter-boundary controls.
4. Connect the discrepancy output only to declared observable boundaries. Any correction to an internal source or flux must supply paired species/reaction, momentum and energy receipts.
5. Add full-family holdouts for geometry, waveform, gas/pressure and experimental session. A residual fit is not a transfer test.
6. Add proper predictive scores and per-cohort interval coverage, and abstain when model support is missing.

The current contribution is a falsifiable D0 reference and a conservation-firewall demonstration, not a claim of predictive capability.
