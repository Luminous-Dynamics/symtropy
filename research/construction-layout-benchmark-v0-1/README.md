# Construction Layout Benchmark v0.1

**Status:** research benchmark profile and synthetic cases. It is not a product certification, legal tolerance standard, safety approval or field result.

## Why layout first

Interior construction layout gives a bounded physical output that can be independently surveyed: partition lines, grid lines, MEP points, sleeves and hanger locations. This makes it a better first field demonstrator than a generic multi-robot site-wide programme.

## Current industry baseline and comparison gap

**Dusty FieldPrinter 2.** Its manufacturer specification, updated 21 September 2026, states ±1/16 in (±1.6 mm) print accuracy given accurate surveyed control, 0.75 m/s continuous line print speed, and a Leica Absolute Laser Tracker positioning architecture. This is configuration-bound manufacturer data, not an independent P95/P99 field result. Source: https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs

**HP SitePrint.** HP publishes layout tolerance ±3/32 in (±2 mm) and floor-level accuracy ±1/32 in (±0.8 mm) on average only under a defined high-accuracy setup: HP SMR prism, High Accuracy Print Mode and a 1-inch total station at 5–30 m. HP's headline "up to 30%" productivity improvement is relative to its previous software version; its official footnote lists 8% for interior-wall layouts. Do not compare that number to a manual workflow saving. Source: https://www.hp.com/us-en/printers/site-print/layout-robot.html

**Published field evidence.** A peer-reviewed 2025 case study of an 8-floor medical office building reported 68% fewer total labour hours, 18% fewer layout days, 99% less physically straining work and rework estimates of 6.42% versus 0.25%. The authors explicitly state that traditional-method impacts were estimated rather than directly observed. The study also reports 204 additional VDC coordination/file-preparation hours in the robotic workflow and 393 robot print-hours. This is valuable real-world evidence, but not a randomized or multi-vendor matched trial. Source: https://doi.org/10.1007/s41693-025-00163-z

**Our competitive challenge.** We should beat the published case-study figures only under a stronger comparison: direct measurement of the manual baseline, same geometry/revision/frame/tolerance, blind independent physical inspection, and all VDC preparation, setup, survey, operation, inspection, blocked work and rework included. The stretch challenge is 70% fewer total workflow crew-hours, 20% shorter elapsed layout duration and 90% fewer rework crew-hours than a directly measured manual baseline. These are ambitious research hypotheses, not achieved results, universal standards or automatic acceptance thresholds.

The benchmark measures total workflow and independently accepted physical output—not robot print speed alone.

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
