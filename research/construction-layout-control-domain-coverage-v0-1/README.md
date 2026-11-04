# Construction Layout Control-Domain Coverage v0.1

**Issue:** [#3569](https://github.com/Luminous-Dynamics/symtropy/issues/3569)  
**Status:** interface/schema proposal; synthetic examples only

## Why this exists

A minimum checkpoint count, large triangle area, or low residual at tested points does not prove the whole construction layout footprint is supported by the independent survey control. Parts of a large or irregular work zone may be outside the control/check network's effective domain, or may lie inside a broad convex hull but have a large unsupported interior gap.

This proposal defines a task-bound coverage assessment that the surveying owner produces and the pre-print layout admission gate consumes. It deliberately does not add a second coordinate engine, survey adjustment implementation or geometry-processing subsystem.

## Research anchors

- FGDC NSSDA prefers comparison against an independent source of higher accuracy, requires the checkpoint distribution to reflect geographic area and expected error distribution, and explicitly says distribution must be application-dependent. For a rectangular area with believed uniform accuracy, its informative example distributes checkpoints at intervals of at least 10% of the diagonal and at least 20% in each quadrant. These are not universal construction-robot acceptance thresholds: https://www.fgdc.gov/standards/projects/accuracy/part3/chapter3
- ISO 17123-5:2018 defines field procedures for evaluating the precision/repeatability of total-station coordinate measurements. It says its procedures are field verification for the immediate task and are not comprehensive acceptance/performance evaluations: https://www.iso.org/standard/71689.html
- The Dusty/manual case study reports an instance where one hour of layout was printed at the wrong location because the supplied CAD file used the wrong coordinate system. That demonstrates why static accuracy does not cure input/reference mistakes: https://link.springer.com/article/10.1007/s41693-025-00163-z

## Ownership

- #1607 owns survey sessions, control network, adjustment, CRS/frame/transform, independent checkpoints, uncertainty and computation of the supported domain.
- #3564 owns the pre-print manifest admission boundary and must bind the assessment to the exact task footprint, model, design revision, frame and site epoch.
- #3523/#3524 own the independent layout-referee comparison.
- #3531 owns claim-specific evidence admissibility.

This proposal's schema is the evidence interface; it does not compute the coverage domain.

## Assessment contents

The contract binds:
- project/site/work-package/task/zone and worksite epoch;
- exact source model and task-footprint geometry digest;
- full target frame semantics and transform-bundle digest;
- survey adjustment, control network, checkpoint set and coverage-domain artifacts;
- fit/check point sets, with explicit disjointness and a reference to the independence assessment;
- supported vs unsupported area, task-footprint relation, max unsupported distance, max control gap, max Delaunay circumradius, minimum support-hull margin and unsupported critical segment count;
- local uncertainty status, maximum local U95 and uncertainty-surface/method references;
- a digest-bound policy with task-specific maximum U95/gap/uncovered area and no-critical-extrapolation requirements;
- producer tool/config identity, independent review state and reason-coded disposition.

## Decision semantics

- `fully_supported`: the whole critical task footprint is covered under the declared task-specific local-support and uncertainty policy.
- `partially_supported`: some of the footprint is unsupported or local support/uncertainty only qualifies a subset.
- `outside_domain`: critical footprint extends beyond the supported domain or requires unapproved extrapolation.
- `indeterminate`: missing, ambiguous, stale or insufficient coverage/uncertainty evidence.
- `not_evaluated`: no coverage evaluation was attempted.

The consuming layout-input admission gate must treat only `fully_supported` as potentially admissible, and only when exact policy and artifact digests match its trusted policy. A good global residual cannot offset an unsupported boundary or local gap. Unknown is not clear.

A convex-hull test alone is insufficient for concave work zones, islands, holes or large sparse interiors. The survey owner must include a local support metric (nearest-control bound, Delaunay gap bound, uncertainty surface, or an explicitly justified alternative). The site-specific risk assessment selects thresholds; this contract invents none.

## Limits

All example identifiers, digests, geometry references and measurements are invented. Schema validation checks shape only. The assessment producer still has to establish independent survey provenance, correct calibration, sound adjustment, meaningful spatial support, and an honest uncertainty model. This proposal is not a field survey, accuracy result, certification or permission to print.
