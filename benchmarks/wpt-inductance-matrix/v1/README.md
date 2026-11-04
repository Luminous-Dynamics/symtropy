# WPT multi-coil inductance-matrix passivity validator v1

**Status:** research software and synthetic known-truth fixtures. No physical WPT system has been qualified by this tool.

## Why this gate exists

For a declared real-valued linear inductive model, coil flux linkages are represented by

\[
\boldsymbol{\lambda}=\mathbf L\mathbf i
\]

and stored magnetic energy is

\[
W_m=\frac12\mathbf i^T\mathbf L\mathbf i.
\]

A reciprocal inductance matrix is symmetric under one consistent coil polarity convention. Passivity requires nonnegative stored energy for every current vector, so the matrix must be positive semidefinite (PSD). A pairwise coupling check

\[
|k_{ij}|=\frac{|L_{ij}|}{\sqrt{L_{ii}L_{jj}}}\le 1
\]

checks only each two-coil subproblem. It does **not** certify the complete matrix when three or more coils interact.

Ansys documents inductance-matrix extraction by sequentially exciting conductors and calculating self/mutual coupling from the field solution. Its transient-solver documentation distinguishes apparent and incremental inductance in nonlinear magnetic systems. See:
- [Ansys: Computing an Inductance Matrix](https://ansyshelp.ansys.com/public/Views/Secured/Electronics/v261/en/Subsystems/Maxwell/Content/Maxwell/ComputinganInductanceMatrix.htm)
- [Ansys: Inductance Computation for 2D and 3D Transient Solutions](https://ansyshelp.ansys.com/public/Views/Secured/Electronics/v261/en/Subsystems/Maxwell/Content/Maxwell/InductanceComputationfor2Dand3DTransientSolutions.htm)

The broader WPT context is multi-transmitter/multi-receiver operation with mutual-coupling and parasitic interactions. This validator is deliberately a small, auditable physics gate before circuit simulation, scheduling or control optimization.

## Input contract

One record represents one complete matrix at one declared operating point:
- 2–32 stable, unique coil IDs in fixed order;
- an explicit dot/polarity convention for every coil;
- matrix kind: `SMALL_SIGNAL` or `DIFFERENTIAL`; secant matrices are rejected because their quadratic form does not generally represent nonlinear stored energy.
- frequency, temperature and one DC-bias current per coil;
- square, finite, real matrix `matrix_h`, in henries;
- same-shape, nonnegative `entrywise_absolute_bound_h` matrix;
- declared absolute and relative numerical tolerances;
- configuration/source references and artifact SHA-256 where evidence is measured or model-derived.

The entrywise bounds are **deterministic absolute error bounds**, not standard deviations or confidence intervals. A probabilistic covariance model must not be relabelled as these bounds without a documented conservative conversion. The uncertainty bound used here is intentionally conservative.

For differential matrices, temperature and DC bias are part of the operating point. The PSD test is a local incremental-stability/convexity check on the differential constitutive matrix; it does not integrate the full nonlinear magnetic energy over a trajectory. A differential matrix from one bias/temperature point must not be reused as a global constant for a saturating magnetic system. A secant matrix is intentionally unsupported: `1/2 i^T L_sec i` is not generally the stored energy of a nonlinear constitutive law. A nonlinear or hysteretic trajectory requires a richer field/circuit model.

## Evaluation

1. Validate dimensions, finite values, positive self-inductances, identifiers and provenance fields.
2. Check each reciprocal pair. Reject if the intervals cannot overlap within the declared numerical tolerance.
3. Form the symmetric midpoint matrix while preserving the maximum observed raw asymmetry in the output. A reciprocal pair receives a conservative symmetric absolute error bound.
4. Compute the smallest eigenvalue of the midpoint matrix with a deterministic cyclic Jacobi method.
5. Compute a conservative matrix spectral-norm error bound using the Frobenius norm of the symmetric entrywise error-bound matrix.
6. Compute the residual Frobenius norm of the remaining off-diagonal Jacobi-transformed matrix. Its spectral norm is bounded by its Frobenius norm, so it is added to the input-data uncertainty before applying Weyl's eigenvalue perturbation bound. The output reports the input uncertainty, Jacobi residual bound and combined eigenvalue uncertainty separately.
7. Classify:
   - `PASSIVE_CERTIFIED`: the lower eigenvalue bound is no less than negative numerical tolerance;
   - `NONPASSIVE_REJECTED`: the upper eigenvalue bound is below negative numerical tolerance;
   - `UNRESOLVED`: the interval overlaps the decision boundary;
   - `NONRECIPROCAL_REJECTED`: at least one reciprocal pair's error intervals do not overlap;
   - `INVALID_INPUT`: structural or semantic input failure.

`UNRESOLVED` is a valid outcome, not a validator crash and not a PASS. No diagonal jitter, eigenvalue clipping, automatic sign flip or silent matrix repair is performed.

## CLI

```sh
cargo run --locked -p wpt-inductance-matrix-validator -- benchmarks/wpt-inductance-matrix/v1/known-truth.jsonl
```

The CLI emits one deterministic JSON result per non-empty JSONL record. If a record contains `expected_disposition`, the CLI checks it against the computed result and exits nonzero on a mismatch. A physical input without that fixture-only field is classified and reported; downstream qualification must require `PASSIVE_CERTIFIED` explicitly.

## Known-truth corpus

The frozen corpus includes:
- positive-definite two-coil matrix;
- singular positive-semidefinite three-coil matrix;
- **pairwise-plausible but globally non-PSD** three-coil matrix (all pairwise |k| = 0.9 while one global eigenvalue is negative);
- clear reciprocity violation;
- reciprocity mismatch contained by the declared entrywise bounds;
- a rank-deficient matrix whose uncertainty interval crosses the PSD boundary.

The pairwise-plausible/global-invalid case is the key negative control. It prevents the weaker check from being mistaken for a physical realizability certificate.

## Numerical regression and property checks

The authored Rust test suite includes deterministic property-style cases in addition to the frozen JSONL corpus:

- 15 seeded Gram-matrix cases from 2 through 16 coils, constructed as A^T A + I and required to remain positive definite;
- the 32-coil supported boundary and rejection immediately above it;
- sub-nanohenry values, where naïvely forming L_ii * L_jj can underflow before the square root;
- extreme diagonal dynamic range, where the geometric-mean coupling denominator must remain finite;
- opposite-sign finite off-diagonal inputs whose subtraction overflows, which must become INVALID_INPUT rather than emit non-finite JSON diagnostics.

The independent NumPy oracle for the scale regressions gives eigenvalues [5e-309, 1.5e-308] H and coupling 0.5 for the sub-nanohenry case; the extreme-range fixture has coupling 1.0. These are numerical regression values, not hardware measurements.

**Numerical caveat:** the Jacobi off-diagonal residual is included in the reported eigenvalue uncertainty. The declared numerical tolerance remains a separate floating-point guard for finite-precision rotation/accumulation; this is not a formal interval-arithmetic proof of the entire eigensolver.
 
**Execution status:** 21 Rust tests are authored, but authored is not executed. Only the exact-head CI run can establish Rust compile/test status. The six JSONL dispositions and added extreme-scale cases have separate external numerical checks; those checks do not substitute for the Rust suite or an independent high-precision library comparison at the full 32-coil limit.

## Published normalized-coupling source audit

`source-normalized-coupling-matrices.json` transcribes Appendix A Tables A1–A5 from Garrofy, Nikulshin and Wolfus (2026), DOI 10.3390/en19163832. The five dimensionless symmetric K matrices cover 3–7 receiver coils plus the transmitter at the aligned point. The source article describes position-dependent K(x) extraction from a nonlinear ferrite FEM model, but the tabulated Appendix A cases are aligned-position matrices.

The Rust regression test checks each source matrix's symmetry, minimum-eigenvalue reference and positive lower margin after a conservative displayed-digit rounding bound. The bound assumes half a unit in the last displayed off-diagonal decimal place and treats the unit diagonal as exact normalization. Expected minimum eigenvalues are retained with each matrix as independent numeric regression references.

Independent NumPy results from the transcribed matrices:

| Appendix | Receivers (+ TX) | Minimum eigenvalue | Frobenius rounding bound | Lower margin |
|---|---:|---:|---:|---:|
| A1 | 3 (+1) | 0.67731315 | 0.00017321 | 0.67713995 |
| A2 | 4 (+1) | 0.44858118 | 0.00022361 | 0.44835757 |
| A3 | 5 (+1) | 0.44855565 | 0.00273861 | 0.44581704 |
| A4 | 6 (+1) | 0.36555122 | 0.00032404 | 0.36522719 |
| A5 | 7 (+1) | 0.36102491 | 0.00037417 | 0.36065075 |

All five have a large positive eigenvalue margin relative to the displayed-digit rounding envelope. These reference values are regression oracles derived from the published rounded tables, not independently measured physical values.

**This is a separate dimensionless audit, not a physical-passivity certificate.** The paper does not type these matrices as SMALL_SIGNAL or DIFFERENTIAL, and supplies no matrix extraction uncertainty/covariance. Do not relabel K as an H-valued L matrix or feed it to the production physical qualification gate. It tests the eigensolver against published multi-coil source data without overstating the source's model semantics.

## Scope and claim ceiling

V1 handles real-valued, reciprocal, passive small-signal or differential inductance matrices at a single declared operating point. It does not qualify:
- complex, frequency-dependent impedance matrices;
- capacitance/conductance/resistance matrices;
- radiation, eddy-current diffusion or dispersive memory effects as a full model;
- secant inductance as an energy quadratic, nonlinear hysteresis or saturation over a trajectory;
- source-data provenance, FEM mesh convergence, geometry correctness or hardware behaviour.

A passing matrix is a necessary consistency condition for this model class, not proof that the underlying field solution or measurement is accurate. Independent reference metrology, field/circuit validation and cross-laboratory replication remain required.
