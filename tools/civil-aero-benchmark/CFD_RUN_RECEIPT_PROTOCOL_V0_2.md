# CFD run receipt protocol v0.2

## Purpose and evidence ceiling

This protocol records enough numerical and resource information to compare solver runs on an explicitly matched steady-CFD grid family. It is a gate over caller-supplied run summaries, not a solver, execution monitor, V&V authority, experiment, airworthiness decision or authenticated run archive.

The current status string deliberately includes `caller_hashes_unverified`. The gate validates the syntax and cross-run consistency of SHA-256 strings; it does **not yet recompute them over the actual solver executable, input files, geometry, mesh, raw solver output or monitor-history bytes**.

## Mandatory per-grid fields

- Stable run and grid IDs, monotonically increasing grid level and explicit characteristic spacing.
- Mesh node count and distinct mesh SHA-256 string.
- Common solver-build, physics-configuration and geometry SHA-256 strings across all members of one study.
- Solver process exit code, required to be zero.
- Positive completed-iteration count.
- Positive CPU core-hours, wall-clock seconds and peak-memory bytes.
- Initial and final aggregate residual values; the final value and reduction ratio must satisfy caller-declared criteria.
- At least six finite samples of the named scalar quantity. The absolute difference between the means of the last two three-sample windows must meet its declared stability threshold.
- Absolute normalized mass and energy balance defects within declared limits.
- Scalar quantity value used by the grid-convergence calculation.

GPU hours and energy consumption are optional only when not measured. When present, they must be finite and non-negative. In CSV, absence is an empty field, never a numeric zero. Study-level totals include the count of runs for which each optional resource was measured.

## Numerical admission

A study requires at least three ordered grids and the GCI module computes every sliding three-grid window. Each window must satisfy the monotone-convergence and observed-order requirements. The asymptotic consistency ratio must meet the caller-declared interval. The GCI module requires `F_s >= 1.25` for three-or-more-grid studies, following NASA's published recommendation.

Passing the gate means only that these supplied scalar diagnostics meet declared criteria. It does not show that all governing-equation residuals converged, that a source file matches its digest, or that the solver is physically accurate. A single scalar residual pair is not a substitute for separately inspecting continuity, momentum, energy and turbulence residual histories. A steady integrated coefficient is not evidence of unsteady buffet or aeroelastic validation.

## Known-answer fixture

The committed six-grid case is synthetic, using `phi(h)=2+0.8h^2`. It exists to test the gate's logic and CSV join behaviour. The known-answer is not CFD output. The deterministic CI demonstration should verify:
- six grid records and four sliding GCI triplets;
- zero solver exit code, non-zero iteration count and mesh-node count;
- CPU-hours, wall time and peak memory on every run;
- GPU/energy measured-run counts and totals;
- explicit blank optional values for runs where those quantities are not measured;
- rejection of a non-zero solver exit, missing resource evidence, non-finite optional measurements, residual failure, unstable coefficient monitor and conservation violations;
- deterministic CSV output and the explicit unverified-hash/non-validation status label.

## Next version: byte-bound run artifacts

The next receipt revision should bind the executable/build, complete input deck, boundary-condition and physical-model configuration, geometry, mesh, solver stdout/stderr, raw result fields, per-equation residual history and force/moment monitor history. Each artifact needs a role, byte length, SHA-256, acquisition method and source URI or execution-run ID. A verifier must hash the actual bytes it parses, not trust a caller-supplied hex string. Preserve the process exit status, timestamps, host/hardware, core/GPU allocation, memory, elapsed time, energy-meter coverage and failed/aborted-run reason codes.

Only after that byte-binding and independent replay should a receipt be considered authenticated at the artifact-integrity layer. Even then it remains numerical solver evidence—not experimental validation or certification.

## References

- NASA CFD verification assessment: https://www.grc.nasa.gov/www/wind/valid/tutorial/verassess.html
- NASA spatial/grid convergence guidance: https://www.grc.nasa.gov/www/wind/valid/tutorial/spatconv.html
- NIST IR 8298, Industrial CFD VVUQ: https://www.nist.gov/publications/summary-industrial-verification-validation-and-uncertainty-quantification-procedures
