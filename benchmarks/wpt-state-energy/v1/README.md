# WPT state-derived energy validator v1

Status: research software and synthetic known-truth fixtures. No physical WPT hardware is qualified.

## Purpose and claim boundary

The hybrid transition validator (#3600) validates scalar event-energy receipts. This crate addresses the next gap: derive those joule values from reciprocal inductance/capacitance matrices and bounded electrical states.

For a linear, reciprocal, quasi-static storage model in explicit SI coordinates:

    H = 1/2 i^T L i + 1/2 v^T C v

L is the full inductance matrix in henries, i is the inductor-current vector in amperes, C is the capacitance/storage matrix in farads and v is the capacitor-voltage vector in volts.

V1 does not apply this formula to nonlinear, hysteretic, dispersive or frequency-dependent materials. Those need constitutive/path-dependent storage models and separate evidence. It also does not certify complete network passivity, dynamic stability, ZVS, thermal safety, exposure, EMC or hardware safety.

## Conservative uncertainty

All abs_uncertainty fields are deterministic absolute bounds, not standard deviations or confidence intervals. Each L and C component also requires a matrix-basis digest and state-basis digest; they must match exactly, binding matrix row/column ordering to the current/voltage vector. Separate matrix, state and calibration receipt SHA-256 values bind the parameter extraction, state measurement and calibration evidence. A digest binds an artifact but does not itself prove the source is calibrated or trustworthy.

For each reciprocal off-diagonal pair, the two entry intervals must overlap. Their intersection supplies the common symmetric interval. The midpoint and radius are used and the maximum reconciliation adjustment is reported. Disjoint reciprocal intervals invalidate the record; values are never silently averaged.

A cyclic Jacobi diagonalization supplies approximate eigenvalue bounds. The remaining off-diagonal Frobenius norm and entrywise matrix-uncertainty Frobenius norm bound the spectral perturbation. Weyl's inequality gives conservative lower and upper bounds for the minimum and maximum eigenvalues. A dimension-scaled 128-epsilon numerical guard is included. If the robust minimum-eigenvalue lower bound is negative but its upper bound is nonnegative, the matrix is unresolved. No eigenvalue clipping or diagonal jitter is permitted.

For a robustly PSD matrix, conservative energy bounds use:

    E_lower = 1/2 lambda_min_lower * ||x||_min^2
    E_upper = 1/2 lambda_max_upper * ||x||_max^2

State norm bounds include every coordinate's absolute uncertainty. Nominal energy is calculated independently from the reconciled nominal matrix and nominal state vector.

## Dispositions

- STORED_ENERGY_DERIVED: both L and C matrices have robust PSD certificates and finite conservative energy intervals.
- STORAGE_MODEL_REJECTED: at least one matrix is provably non-PSD under its uncertainty bounds.
- STORAGE_MODEL_UNRESOLVED: eigensolver non-convergence or PSD interval overlap with zero.
- INVALID_INPUT: bad units, dimensions, digest, nonfinite arithmetic, unsupported model class or incompatible reciprocal intervals.

An exact diagonal singular PSD matrix is directly admissible. Near-singular non-diagonal matrices whose uncertainty interval overlaps zero must abstain.

## Frozen corpus and tests

The JSONL corpus contains analytic energy, singular PSD, indefinite, PSD-boundary uncertainty, reciprocal interval overlap/conflict, dimensional mismatch, state uncertainty, tiny-scale and high-dynamic-range cases. Seeded Gram-matrix tests cover dimensions 2–16. The CLI checks expected dispositions and rejects duplicate record identifiers.

## Run

    cargo test --locked -p wpt-state-energy-validator
    cargo run --locked -p wpt-state-energy-validator -- benchmarks/wpt-state-energy/v1/known-truth.jsonl

## Integration path

1. Emit a state-storage certificate for each pre/post mode in #3600.
2. Bind the L/C matrix, state-vector, reciprocity-reconciliation and energy-interval evidence to the transition receipt.
3. Reconstruct pre/post energy independently and reconcile the interval difference against port work, switching work, dissipation and energy-tank debit/credit.
4. Extend later versions to nonlinear constitutive storage and frequency-dependent positive-real models only when suitable storage functions and evidence are available.

## Prior art

Frasca et al., IEEE TCAS-I (2010), DOI 10.1109/TCSI.2010.2052511: energy-minimizing state reinitialization for passive switched electrical networks.

Restrepo et al., Automatica 181 (2025), DOI 10.1016/j.automatica.2025.112496: passivity-preserving hybrid energy tanks for switched systems with changing state dimension.

Lee & Lee, Scientific Reports 16, 19957 (2026), DOI 10.1038/s41598-026-50781-y: heterogeneous multi-receiver EM-to-circuit WPT digital twin, with fixed-layout/steady-state limits that motivate state and spatial extensions.

The v1 contribution is WPT-specific, uncertainty-bounded storage reconstruction and reproducible evidence—not a claim to invent quadratic storage or hybrid passivity.
