# Exact-rational operator spectrum oracle v0.1

This package is the first calibration lane for [PHYS-METRIC-SPECTRUM-ORACLE-001 #5378](https://github.com/Luminous-Dynamics/symtropy/issues/5378), stacked on the square and rectangular operator implementations in [#4820](https://github.com/Luminous-Dynamics/symtropy/pull/4820) and [#4866](https://github.com/Luminous-Dynamics/symtropy/pull/4866).

## What it checks

The Rust reference computes singular values using a one-sided Jacobi SVD of the metric-whitened operator. This independent lane uses frozen integer matrices, exact rational rank and characteristic polynomials, and rational real-root isolation of the smaller Gram matrix. The isolated nonnegative eigenvalue intervals are converted to outward-rounded decimal singular-value intervals.

The comparison checks singular values, exact numerical rank, domain/codomain nullities, status, whole-map condition availability, and the nonzero-spectrum condition separately.

## Frozen cases

- 2×2 weighted diagonal map with exact singular values 4 and 3;
- dense integer 2×2 map with nontrivial irrational singular values;
- 3×3 repeated/clustered singular value;
- exact rank-deficient square map;
- badly scaled full-rank 3×3 diagonal map with spectrum 10¹², 10⁶, 1;
- 2×3 full-row-rank wide map, where domain nullity is one and no whole-map condition number exists;
- 3×2 full-column-rank tall map, where the map is injective and its whole-map condition is defined;
- rank-deficient 2×3 wide map.

## Reproduction

From the repository root, with SymPy 1.14.0 installed:

    python3 research/operator-spectrum-oracle-001/operator_spectrum_oracle.py --input research/operator-spectrum-oracle-001/operator-spectrum-inputs-v0.1.json --check-fixture

The exact-head workflow runs the Rust tests and strict Clippy, emits candidate receipts through `operator-spectrum-probe`, regenerates rational root enclosures, compares the candidate and uploads the oracle/candidate/validation reports.

## Why the Gram matrix is only an oracle

For A, the smaller of AᵀA and AAᵀ has the same nonzero singular values squared. This is convenient for exact rational characteristic-polynomial construction on tiny integer fixtures, but it squares the nonzero spectral condition number. This path is therefore an independent small-case oracle only; it is not a replacement for the one-sided Jacobi implementation and is not suitable for calibrating high-condition large matrices by itself.

The oracle uses exact rational arithmetic for rank and the characteristic polynomial, with rational intervals isolating the real roots. It does not use floating-point NumPy SVD as its reference. Its decimal intervals are outward-rounded; the Rust f64 candidate is compared with an explicit, wider, preregistered relative tolerance.

## Acceptance criteria

- exact rank agreement on every frozen fixture;
- all candidate singular values fall inside their exact-rational enclosures widened by the declared f64 comparison tolerance;
- no finite whole-map condition claim for rank-deficient or non-injective maps;
- full-row-rank wide map reports nonzero-spectrum condition but no whole-map condition;
- full-column-rank tall map reports a finite whole-map condition;
- machine-readable reports identify the exact source commit and oracle parameters.

These are bounded calibration tests, not a formal proof of the Jacobi SVD for arbitrary floating-point matrices. The next lane is an Arb/FLINT ball-arithmetic oracle for near-rank-deficient and clustered spectra, with risk/coverage calibration across condition tiers.
