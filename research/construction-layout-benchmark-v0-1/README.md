# Construction Layout Benchmark v0.1

**Status:** research benchmark profile and synthetic cases. It is not a product certification, legal tolerance standard, safety approval or field result.

## Why layout first

Interior construction layout gives a bounded physical output that can be independently surveyed: partition lines, grid lines, MEP points, sleeves and hanger locations. This makes it a better first field demonstrator than a generic multi-robot site-wide programme.

Current supplier specifications are useful starting baselines but remain supplier claims:
- Dusty FieldPrinter 2: ±1.6 mm under its stated surveyed-control conditions: https://www.dustyrobotics.com/support/fieldprinter-2-specs
- HP SitePrint: ±2 mm tolerance and ±0.8 mm average under a specified high-accuracy configuration and total-station/prism setup: https://siteprint.hp.com/
- A 2025 peer-reviewed single-project study reported 68% fewer labour hours and 18% fewer days for a Dusty deployment compared with an estimated manual baseline; it is not a multi-vendor randomized trial and should not be generalized as a universal causal estimate: https://doi.org/10.1007/s41693-025-00162-6

The study also illustrates why total-workflow accounting matters: model coordination, file preparation, survey/control setup, line-of-sight, obstructions, weather, supervision, inspection and rework all affect the result.

## Non-negotiable benchmark design

1. Freeze the approved design revision, geometry, target coordinate frame, transform bundle, site epoch, control network and project acceptance profile.
2. Use matched geometry and randomized blocks/crossover cells to compare methods. Record differences in access, substrate and obstruction conditions.
3. Have an independent survey professional qualify instruments, calibration, control and acceptance checkpoints. Report measurement uncertainty; do not infer physical accuracy from robot odometry or print counters.
4. Keep every planned feature in the denominator. Report accessible scope and total planned scope separately; blocked/deferred features remain visible.
5. Report both robot-only productivity and total-workflow crew productivity. Include VDC/model preparation, conversion, control setup, operation, spotter, QA, inspection, recovery and rework.
6. Do not silently substitute a vendor advertised tolerance for the project tolerance. The governing contract/engineer/surveyor-approved acceptance profile remains authoritative.
7. Estimate trial sample size from pilot variance and predeclared effect size before confirmatory comparison.

## Scorecard

The machine-readable profile defines per-feature P50/P95/P99/max error, signed bias, line geometry, omissions/duplicates, uncertainty, accessible and total coverage, rework hours, utilization, integration effort, downtime and full lifecycle cost per finally accepted unit.

Proposed stretch hypotheses (not standards or achieved values):
- controlled indoor P95 position error ≤ 1.0 mm and P99 ≤ 2.0 mm, only where the project tolerance and instrument uncertainty support that result;
- expanded measurement uncertainty at k=2 ≤ one-third of governing tolerance;
- ≥30% lower total workflow crew-hours per accepted unit than a matched manual baseline;
- ≥50% fewer rework crew-hours;
- ≥95% accepted accessible scope;
- ≥20% lower fully scoped lifecycle cost per accepted unit.

These are hypotheses for research planning. Do not treat them as automatic pass/fail standards across different tasks, sites or contractual tolerances.

## South African application

Use the project contract, client/authority requirements and the responsible engineer/registered survey professional's approved set-out and tolerance plan. Road/civil survey guidance (for example the Department of Transport or TMH11) may be relevant to those work classes; it is not a universal tolerance specification for interior building layout. Verify the applicable edition and scope before use.

## Frozen synthetic corpus

15 synthetic cases cover valid matched context, stale design/epoch, frame and transform mismatches, inadequate uncertainty, omitted preparation time, denominator manipulation, vendor-tolerance substitution, executor FINISHED without physical/inspection receipts, mismatched manual geometry, missing calibration evidence, lossy unit conversion, scope denominator separation and incomplete lifecycle accounting.

The focused workflow validates profile/schema and fixture/schema parity and ensures case IDs are unique. Passing this corpus establishes only consistency of the benchmark definition.
