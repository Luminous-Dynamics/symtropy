# Radiance convergence sweep protocol v0.1

**Status:** preregistration template; no solver sweep or result is included.

## Aim

Select a fixed, defensible set of Radiance calculation settings by measuring numerical stability against compute cost. Do not choose solver settings separately per algorithm after seeing the blind comparison results. The purpose is to prevent an optimizer from appearing superior because it was given a more expensive solver budget or because a flattering run configuration was selected post hoc.

## Freeze before the sweep

- Official toolchain/source URL, actual source archive SHA-256, build environment and executable digests.
- Case/scene/material/luminaire/source-property hashes and task-grid coordinates.
- Evaluation endpoints: per-point lux, mean/minimum/maximum, minimum-to-average uniformity, design ranking stability and runtime/CPU budget.
- A ladder of ambient and direct sampling settings, convergence criteria and maximum compute budget.
- Independent scenes used only for convergence selection; these must not be reused as the blind confirmatory test family.

## Procedure

1. Verify source/tool identity, build the octree from frozen scene inputs, and record the exact argv and runtime environment.
2. Run a predeclared low-to-high ladder of sampling settings on each convergence scene, using repeated seeds where the solver contains stochastic sampling. Preserve every attempt and failure.
3. Compare each level with the next higher fidelity level at every task-plane grid point and on the derived grid metrics. Report absolute and relative errors; handle near-zero denominators explicitly.
4. Test rank stability across a fixed set of candidate designs. A setting level is not adequate merely because the average lux changes little if a minimum, uniformity gate or design ranking changes materially.
5. Choose the least expensive setting that meets the frozen pointwise and ranking stability tolerances on all required convergence scenes. Report failures and worst-case behavior instead of averaging away a problematic family.
6. Freeze the selected settings before opening the blind scene set; use exactly the same policy for all methods and charge every high-fidelity call to the matched budget.

## Required output

Publish scene/tool/config digests, exact argv, seeds, raw output digests, normalized per-point samples, per-level runtime, pointwise errors, grid-metric deltas, design rank stability, failures and the predeclared pass/fail decision. The benchmark runner's process receipt is not a convergence certificate; the analysis script must recompute the comparison from the recorded samples.

## Acceptance gates

The numerical tolerances must be application-specific and preregistered. Candidate criteria can include maximum absolute lux delta, relative error above an explicit minimum-light threshold, change in uniformity, and no change in compliance classification. Set no universal tolerance after seeing the sweep. If no setting level meets the criteria within the compute budget, report unresolved numerical accuracy and do not compare optimizer winners using that solver.

## Evidence ceiling

This procedure chooses numerical settings for a specified simulation corpus. It does not establish that Radiance matches physical data, that the luminaire spectrum/source model is correct, or that a design complies with an applicable standard. Those require separate source validation and measurement evidence.
