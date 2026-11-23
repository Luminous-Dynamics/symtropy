# EHD-OED-002 — nuisance-adjusted robust experiment portfolio frontier

**Issue:** #7036  
**Parent:** EHD-OED-001 / #6660  
**Implementation:** `tools/ehd-oed-portfolio/`  
**Status:** reference source and manufactured tests authored; Rust format/build/test/Clippy pending exact-head CI.

## Paradigm shift

A single “best next experiment” score is an incomplete description of scientific value. A portfolio must jointly resolve physical parameters, distinguish competing explanations, include controls that expose artifacts, and preserve enough diversity to make the result interpretable.

The reference therefore returns a Pareto frontier rather than collapsing incompatible objectives into one scalar. The design objectives are:
- worst-case nuisance-adjusted physical information;
- worst-case nuisance-profiled model separation;
- log determinant as a complementary volume criterion;
- experimental cost.

A design cannot compensate for a missing sham, reversal or independent diagnostic by scoring well on information alone.

## Nuisance-adjusted information

The design has two physical and two nuisance parameters for a manufactured linear-Gaussian test model. For the selected portfolio, the engine forms the full joint information matrix using the inverse of the complete candidate-level measurement covariance matrix:

[
I = egin{bmatrix}
X_p^TSigma^{-1}X_p & X_p^TSigma^{-1}X_n \
X_n^TSigma^{-1}X_p & X_n^TSigma^{-1}X_n
end{bmatrix}.
]

The physical information remaining after the nuisance directions are profiled out is:

[
I_{pmid n}=I_{pp}-I_{pn}I_{nn}^{-1}I_{np}.
]

The nuisance block must be positive definite at a declared relative determinant tolerance. This version does not add a ridge term or use a pseudoinverse; singular nuisance information rejects the portfolio.

## Nuisance-profiled model discrimination

For each model pair, the engine constructs the predicted mean-difference vector (delta) across selected observations. It evaluates the weighted residual after fitting the difference explainable by the nuisance directions:

[
D^2=delta^TSigma^{-1}delta
-(X_n^TSigma^{-1}delta)^T
I_{nn}^{-1}(X_n^TSigma^{-1}delta).
]

A pair that differs only in an offset/drift/reference nuisance direction has zero residual separation and must not be declared discriminable.

## Correlated errors and scale invariance

The full measurement covariance is accepted per robustness scenario. This allows shared reference, camera calibration, clock, illumination or other common-mode effects to induce off-diagonal covariance between candidate experiments.

Before matrix factorization, the implementation transforms the covariance to a dimensionless correlation matrix and scales the sensitivity/prediction rows by their standard deviations. This keeps the calculation invariant under independent observation-unit rescaling. The original covariance is not silently repaired: only asymmetry within a frozen relative tolerance is averaged to form its symmetric numerical representation; material asymmetry or non-positive-definiteness is rejected.

## Robust scenarios

Each candidate carries physical sensitivity and model-prediction values for each declared scenario. Each scenario also has a full measurement covariance matrix. The portfolio score uses minima over scenarios, not a nominal average. Scenario records should represent predeclared uncertainty ranges, not be selected after results are observed.

## Portfolio selection

Every non-empty subset is considered up to a hard limit of 16 candidate experiments. It must satisfy:
- cost budget;
- required control tags;
- minimum number of distinct condition families;
- full-rank nuisance information;
- full-rank nuisance-adjusted physical information;
- predeclared minimum model-pair separation.

The surviving portfolios are filtered by Pareto dominance across E-like information, D-like information, model separation and cost. The result is a frontier; no arbitrary scalar weighting chooses one hidden winner.

## Manufactured controls authored

- nuisance profiling removes apparently full-rank physical-only information;
- model predictions that differ only in the nuisance subspace are not discriminated;
- shared measurement covariance changes the profiled separation;
- a singular nuisance block fails closed;
- missing required control tags abstain;
- worst-case scenario metrics are not replaced by nominal averages;
- candidate-order permutation preserves scores and frontier identities;
- independent unit rescaling preserves qualification;
- singular/asymmetric covariance is rejected;
- the returned frontier contains no dominated pairs;
- candidate count above the frozen exhaustive-search ceiling is rejected.

## Benchmark comparison

The comparator set should include the existing two-parameter D-optimal and E-optimal tournament, a random design, a balanced control-first design, and the new nuisance-adjusted Pareto frontier. All must use the same candidate set, observation budget, covariance scenarios and control-coverage constraints.

A useful future extension is to compare the linear local sensitivity scores with nonlinear profile likelihood and posterior predictive model discrimination on real solver adapters. That is outside v0.1.

## SOTA and industry boundary

PIVtools (Taylor, Lawson & Ganapathisubramani, 2026, SoftwareX 35, 102967) is an end-to-end PIV processing comparator with planar/stereo/ensemble pipelines and DNS-referenced turbulent-channel validation. This work does not compete with its image correlation, Reynolds-stress extraction or HPC throughput. Its purpose is orthogonal: qualify whether the experiment portfolio can distinguish physical and nuisance explanations before an engineering claim is attempted.

Liu, Maini & Baker (2026, Mathematical Biosciences 399, 109710) provide a current method comparator for profile-likelihood practical identifiability and optimal-control model discrimination. NISTIR 8298 provides the broader industrial CFD V&V/UQ credibility baseline.

No physical EHD model, operating point, thrust estimate, or experimental authorization is emitted by this crate.
