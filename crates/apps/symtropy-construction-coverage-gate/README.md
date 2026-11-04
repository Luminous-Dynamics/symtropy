# Construction control-domain coverage admission gate

This Rust library consumes the provenance-bound survey evidence contract from `symtropy-robotics-bridge-core`. It checks whether a survey owner's coverage assessment supports one exact construction layout task under an explicitly supplied project policy.

## What it checks

- Exact survey-envelope artifact identity and content-addressed verification.
- Exact control-adjustment receipt identity, stage, disposition, output-domain binding and independent checkpoint set.
- Exact site, design revision, work-package revision, task, work zone, worksite epoch, target frame and task-footprint artifact.
- Exact control-domain artifact and pinned coverage algorithm/build/method identifiers from trusted policy.
- Reconciled task-footprint area and spatial cell denominators in the upstream coverage assessment.
- Separate policy bounds for unsupported area, unmeasured area, critical uncovered area, maximum support gap/distance, minimum local control density and maximum local U95.

The gate computes one decision for the named task: `admissible_for_named_task`, `inadmissible_for_named_task`, or `indeterminate`. Reason codes remain independent. There is no weighted score, averaged residual or global safety PASS.

## Strict ownership boundary

The survey owner computes the control-support domain, footprint overlay, support map, local support density/gap and local uncertainty. The gate does **not** compute CRS transforms, survey adjustment, convex hulls, polygon intersections, interpolation, or a new support-domain engine.

A global convex-hull inclusion test is not treated as sufficient. The assessment carries a spatial support map and maximum local gap/distance and local density/U95 summaries. The trusted policy decides which metrics and thresholds are necessary for that specific task/risk context.

## Trust boundary

The policy is supplied by the project/site authority. The gate verifies its exact serialized bytes against the separately supplied policy artifact digest, but does not authenticate the publisher or verify a digital signature. Production callers must obtain policy bytes and artifact identity from an independently authenticated authority channel.

Likewise, content hashes establish byte identity, not truth. The gate verifies the receipt structure and bindings, but does not validate surveyor competence, instrument calibration physically, the accuracy of the support-domain algorithm, the physical control network, or whether an independently measured support map matches reality. Those remain upstream/independent evidence responsibilities.

## Run

```sh
cargo test -p symtropy-construction-coverage-gate
```

All included examples are synthetic fixtures. Tests prove software-contract behavior only; no field survey, print authorization, accuracy qualification, certification, or physical performance is claimed.
