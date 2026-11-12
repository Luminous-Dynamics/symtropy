# Registered profile geometry oracle v0.1

Tracking: [SHIPYARD-VERIFICATION-RECOVERY-001 #4813](https://github.com/Luminous-Dynamics/symtropy/issues/4813)

## Purpose

The inter-pass planner needs a reproducible measurement contract between an observed cross-section and a reference profile. This dependency-free Rust library provides that contract for already-registered **2D cross-section profiles**.

It does not register point clouds or ingest BAM NetCDF/HDF5 files. It verifies that the inputs claim the same workpiece/joint/frame and that the registration receipt binds the exact reference and scan digests. The output is a conservative deviation-area bound in symbolic coordinate units.

## Metric

At each matched station i, let r_i be the reference height, m_i the measured height, u_i the measured vertical uncertainty bound, and e_r the registration error bound.

The endpoint error envelope is:

E_i = |r_i - m_i| + u_i + e_r

For a valid adjacent interval of width Δx_i, twice the trapezoidal error-area upper bound is:

A_i^(2) = Δx_i(E_i + E_(i+1))

The reported value is the checked integer sum over valid adjacent intervals. It is twice the area so the implementation avoids floating-point rounding. The interpolation assumption is explicit: piecewise-linear between valid adjacent samples only. This is a bound for that profile model, not a guarantee about unsampled continuous surface features.

Missing samples do not get interpolated. An interval touching an invalid measured station is skipped; coverage is the sum of valid adjacent interval lengths divided by total profile length. The comparison is rejected if coverage is below the declared threshold.

## Evidence identity and failure conditions

Inputs must bind:
- workpiece ID, joint revision and coordinate frame;
- reference profile SHA-256;
- measured scan SHA-256;
- registration receipt SHA-256;
- registration receipt's source scan and reference digests;
- transform revision and registration error bound;
- common strictly increasing station grid and maximum sample gap.

The oracle rejects missing/zero digests, mismatched receipt sources, frame/revision mismatches, non-monotonic or mismatched station grids, excessive gaps, too few points, insufficient coverage, no covered intervals and checked arithmetic overflow.

## Tests and boundaries

The 19 unit tests include a frozen 2,800 twice-area known answer, exact zero-deviation case, uncertainty composition, source-receipt binding, missing digest, station-grid/frame/revision mismatch, missingness/coverage, maximum gap and overflow controls.

All coordinates and objective outputs in v0.1 are synthetic symbolic units. This is not a 3D surface reconstruction, weld-volume estimator, bead-fusion predictor, WPS/PQR/WPQT qualification, NDT result, weld acceptance or final free-state distortion metric.
