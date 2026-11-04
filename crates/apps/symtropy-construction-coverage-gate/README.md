# Construction control-domain coverage admission gate

This Rust library consumes the provenance-bound survey evidence types from `symtropy-robotics-bridge-core`. It checks whether one exact construction layout task is supported by the current worksite survey/control domain under an explicit project/site policy.

## What it checks

- Exact survey-envelope digest and source control-adjustment receipt bytes, qualified adjustment stage, output-domain binding, independent checkpoint records, and target frame.
- Exact site, design revision, work-package revision, task, work zone, worksite epoch, target frame, and task-footprint artifact.
- Exact support-map artifact bytes and bindings to the footprint digest, control-domain digest, epoch, frame, support-domain method, and uncertainty method.
- Independent recomputation of per-cell area/count totals, critical uncovered area, maximum local support gap/distance, minimum local control density, and maximum local U95. The assessment summary must match the support-map cells.
- Separate policy limits for unsupported/unmeasured area, critical uncovered area, maximum support gap/distance, minimum local control density, and maximum local U95.

The gate computes one decision for the named task: `admissible_for_named_task`, `inadmissible_for_named_task`, or `indeterminate`. Reason codes remain independent. There is no weighted score, averaged residual, or global safety PASS.

## Paradigm shift: evidence at every location, not a global hull or a single RMSE

A set of widely separated checkpoints and a low global RMSE do not prove local control support across a whole print footprint. A global convex hull can also conceal long sparse interior gaps, holes, concavities, narrow corridors, or disconnected regions.

The survey owner provides a content-addressed `SurveyControlCoverageMapV1`. Each cell is explicitly classified `supported`, `unsupported`, or `unmeasured`, includes its clipped footprint area and criticality, and carries local support gap, distance-to-control, qualifying-control density, and local U95 when measured. An unmeasured cell is not permitted to carry inferred metric values. The consumer recomputes summaries from these rows and rejects a summary that does not match.

This v0.1 map stores per-cell measurements and area but not cell polygon vertices or a full geometric overlay. The map is bound to the exact task-footprint and control-domain artifact digests; the survey producer remains responsible for the actual geometry, clipping, and physical derivation. Independent inspection of spatial cell geometry and external reference measurements remain necessary before physical qualification.

## Strict ownership boundary

The survey owner computes the control-support domain, footprint overlay, clipped support cells/map, local support and uncertainty. The gate does **not** compute CRS transforms, survey adjustment, convex hulls, polygon intersections, interpolation, or a new support-domain engine.

The gate only checks exact artifact and context binding, validates cell data, recomputes declared summary statistics, and applies the task-specific policy. This separation avoids a second survey/geodesy implementation.

## Trust boundary

The policy is supplied by the project/site authority. The gate verifies its exact serialized bytes against a separately supplied policy ArtifactRef digest, but does not authenticate the publisher or verify a digital signature. Production callers must obtain policy bytes and artifact identity from an independently authenticated authority channel.

Content hashes establish byte identity, not truth. The gate does not physically calibrate instruments, assess surveyor competence, validate the support-domain algorithm against external ground truth, or prove that the survey producer's clipping geometry reflects the real site. Those remain upstream and independent-assessor responsibilities.

## Run

```sh
cargo test -p symtropy-construction-coverage-gate
```

All included examples are synthetic fixtures. Tests prove software-contract behavior only; no field survey, print authorization, accuracy qualification, certification, or physical performance is claimed.
