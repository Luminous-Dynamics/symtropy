# CFD evidence admission protocol v0.1

## Purpose and evidence ceiling

This layer is an admission gate for steady CFD numerical evidence. It binds several per-grid run records to one case, quantity, solver build, physics configuration and geometry identity; checks iterative convergence, monitor stability and global mass/energy defects; and evaluates a separate three-grid GCI study.

A passing report is labelled numerically admitted under declared criteria. It is not experimental validation, proof that a turbulence/transition model is correct, flightworthiness, certification, or proof that the data source is authentic. Caller-supplied hashes are identity assertions; without an authenticated artifact store or signature they do not prove who supplied the bytes.

Use a separate invocation for each scalar quantity (for example CL, CD and CM). Do not use a CD study's convergence order to admit CL or CM.

## Required per-grid record

Each coarse-to-fine run supplies a run ID, grid ID/level, explicit characteristic spacing, mesh digest, solver build digest, shared physics-configuration digest, shared geometry digest, explicit iterative-convergence status, initial/final residual norm, at least six final monitored coefficient samples, normalized global mass and energy imbalance fractions, and the audited scalar coefficient.

The mesh digest must differ for each grid. Solver build, physics configuration and geometry digests must be identical across the refinement family. Grid levels must strictly increase while characteristic spacing strictly decreases.

## Declared criteria

The caller must supply and retain:
- maximum final residual norm;
- maximum final/initial residual ratio;
- maximum absolute difference between the means of the final two three-sample coefficient windows;
- maximum mass and energy imbalance fractions;
- observed-order interval for each quantity;
- asymptotic GCI ratio interval;
- GCI safety factor and characteristic-spacing definition.

Criteria are data, not hidden defaults. Reports retain them so the verifier can inspect what the caller asserted. Criteria must be fixed before the solver results are reviewed. A broad or retroactively changed threshold invalidates the comparison.

## Admission checks

A run family is rejected if any identity digest is malformed or inconsistent, a mesh is reused, a grid ordering is invalid, a run is not marked converged, residual levels/reduction fail, the monitored coefficient is unstable, or the mass/energy balance exceeds declared limits.

After all per-grid checks pass, the same scalar outputs are passed through the GCI module. Every sliding three-grid window must have an observed order and asymptotic ratio inside the predeclared intervals.

The algorithm does not assert that a solver residual threshold is universally appropriate; the project/test case owner must declare the proper criterion. It does not assume that a residual decrease alone means a force coefficient is converged. The separate quantity-history test exists to catch this common failure.

## Benchmark programme

For DPW8 OAT15A V1a, run separate admission studies for CL, CD and CM across the recommended six-member family. Before admitting the actual case, establish and document a valid characteristic-spacing definition for that exact mesh family; node count is insufficient. Preserve each grid's solver logs, geometry/mesh/config digests, flow conditions, force histories and conservation checks. For an unsteady buffet case, this steady monitor gate is not sufficient; introduce time-step convergence, statistically stationary windows, spectral uncertainty and phase-sensitive comparisons as a separate module.

A passing numerical gate is only one rung before experimental validation. The next stage compares each quantity and pressure distribution against the correct experimental reference and uncertainty, with model-form sensitivity reported separately.

## Synthetic tests

The CLI and Rust tests use an exact synthetic power law over six explicitly supplied spacings. They prove gate arithmetic and rejection behaviour only. No physical CFD data is included or implied.
