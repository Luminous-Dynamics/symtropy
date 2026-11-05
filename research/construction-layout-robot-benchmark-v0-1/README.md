# Construction Layout Robot Benchmark V0.1

**Issue:** [#3523](https://github.com/Luminous-Dynamics/symtropy/issues/3523)  
**Profile:** `construction-layout-printing-common-reference-v0.1`  
**Status:** research benchmark profile; not executed against physical products.

## Why this profile first

Autonomous construction layout provides a tractable first physical qualification task. Geometry can be generated from frozen design files, outputs can be independently surveyed, and product-specific setups can be reproduced without beginning with hazardous powered tools or earthmoving.

The benchmark deliberately separates:

1. **Vendor-configuration reproduction:** replicate each manufacturer's specified control, equipment, range and environment, with those exact preconditions recorded.
2. **Common independent referee:** compare both outputs on identical frozen layouts and ODD strata using a calibrated reference not used by either robot for control or navigation.

A result from one track does not substitute for the other.

## Industry references

### HP SitePrint

HP reports ±2 mm layout tolerance and average floor-level accuracy of ±0.8 mm under a specified high-accuracy configuration: SMR prism, High Accuracy Print Mode and 1-inch total station, within a stated 5–30 m range. HP also says its productivity claims are based on internal testing and vary by application. See [HP's product page](https://www.hp.com/us-en/printers/site-print/layout-robot.html).

### Dusty FieldPrinter 2

The manufacturer's September 21, 2026 specification states ±1.6 mm print accuracy given accurate surveyed control; three forward time-of-flight obstacle sensors with approximately 0.9 m range; a 5.8° maximum slab slope; IP51-equivalent protection; and no rain operation. See [Dusty's technical specification](https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs).

These are manufacturer statements under different configurations and conditions, not matched independent benchmark measurements. The profile records them as `vendor_reported`, never as observed result data.

## Worksite and trade-coordination ODD

Construction layout is not performed in a stationary factory cell. Record changes to work-zone boundaries, concurrent plant and trade activity, temporary exclusion zones, access routes, work phase and design-change notices. The benchmark may record that a crew briefing or site coordination notice occurred, but must not infer worker identity, intent or affect from cameras or telemetry, and should avoid collecting personal data that is not necessary for the safety/task claim. This is motivated by NIOSH's 7 October 2026 construction robotics guidance on changing work areas, multiple employers/trades, unfamiliar workers, and continuing risk monitoring.

## Required measurements

- Signed X/Y and absolute plan-position residuals at independent checkpoints.
- Orientation, feature completeness, omissions, duplicates, local warp and length/angle error.
- p50/p95/p99/max residuals and uncertainty budget by ODD stratum.
- Obstacle encounter detection, swept-volume clearance, stop/brake latencies and intervention rates in a separately safeguarded test track.
- Accepted output per elapsed hour, crew-hours per accepted output, setup, re-stationing, rework and material consumption.

The robot's own control points must not also be used as the independent survey checkpoints. Reuse #1607 coordinate-frame and survey-adjustment semantics.

## Frozen synthetic geometry descriptors

`fixtures/layout-strata.synthetic.json` defines five invented layout descriptors: sparse orthogonal, dense MEP, curvilinear, edge/obstacle-adjacent, and station-transition. All counts are explicitly synthetic; they are not physical measurements or vendor layouts.

## Targets

Prospective challenge targets are p95 residual <=1.0 mm, p99 <=2.0 mm, zero omissions of critical features, and at least 20% fewer crew-hours per accepted output than a matched manual baseline. These are aspirational research targets, not achieved product performance, universal tolerances or certification criteria. A qualified metrology setup must resolve the decision margins, and all task-specific safety gates remain independent hard gates.


## State-of-the-art challenge and the evidence gap

The 2025 single-project study provides a useful challenge reference, but its baseline must be interpreted carefully. For one 8-floor medical-office building it reported 68% fewer total labour hours, 18% fewer layout days, 99% less physically straining work and rework estimates moving from 6.42% to 0.25%. The authors state that the traditional-method impacts were estimated rather than directly observed. The robotic workflow also used 204 additional hours of VDC coordination and file preparation, alongside 393 robot print-hours. These figures are contextual single-project evidence, not a matched multi-vendor causal estimate.

HP's product page advertises "up to 30%" faster point printing relative to its previous software release; its footnote reports 8% for interior-wall layouts. That is not a 30% improvement over manual layout. Keep software-version uplift, vendor-configured performance and manual-baseline productivity as separate comparisons.

**Prospective SOTA challenge hypotheses (not achieved results):**
- at least 70% fewer *total workflow crew-hours* per finally accepted unit than a directly measured, matched manual baseline;
- at least 20% shorter elapsed layout duration under the same matched work package;
- at least 90% fewer rework crew-hours.

These intentionally seek to exceed the case-study's 68% labour / 18% duration figures under a stronger trial design. They require direct observation of the manual comparator and inclusion of all VDC/model preparation, design coordination, survey/control setup, export/transform QA, robot setup, spotter, inspection, downtime, interruptions, blocked scope and rework. They are research stretch hypotheses, not standards, universal thresholds or claims of achieved performance.

For the confirmatory trial, the independent experimental unit is a matched work-zone/run (not individual printed vertices or repeated measurement frames). Estimate the sample size from a pilot variance and predeclare the practically meaningful effect before unblinding. Preserve every attempted run, failed setup, interruption, invalid/indeterminate output and blocked feature in the report. Use a second site and independent referee replication before transferring the result.

## Evidence and ownership

- #3505/#3506 own physical validation protocol and ODD/metrology framework.
- #3521/#3522 own trial receipt schema and semantic admissibility.
- #1607 owns survey provenance, coordinate frame and uncertainty semantics.
- #1595 owns shared scenario identities and fault schedules.
- #3317/#3499/#2672 own snapshot, temporal and mission safety evidence respectively.

Every physical run should produce a #3522 trial receipt bound to the design revision, robot configuration, site snapshot, ODD, survey artifacts, reference instruments and independent adjudication.

## Frozen adversarial corpus

`fixtures/layout-benchmark-adversarial-v0.1.json` contains 15 synthetic cases. It protects against stale design/site context, frame/transform mismatch, inadequate measurement uncertainty, omitted VDC/preparation time, denominator manipulation by dropping blocked work, substituting vendor tolerance for project tolerance, treating executor FINISHED as independent acceptance, mismatched manual geometry, missing instrument calibration, lossy unit conversion and incomplete lifecycle-cost accounting.

The closed Draft 2020-12 case schema and `scripts/validate_benchmark.py` verify case identity, scorecard coverage, vendor evidence labels and SOTA challenge fields. The focused workflow runs independently of workspace CI. Passing these checks validates benchmark artifact consistency only; it does not report a physical robot result.

## Non-goals

No universal robot ranking, no cross-task composite, no use of a vendor's own positioning output as independent ground truth, no inference of safety from positioning accuracy, and no claim of standards conformity or certification.


## Industry baseline

See [vendor-baseline-matrix.md](vendor-baseline-matrix.md) for a task-class-separated comparison of HP SitePrint, Dusty FieldPrinter 2, CivDot, CivDot+ and CivDot Mini, including vendor caveats, matched tracks, and standards scope.

## End-to-end workflow benchmark and pre-print input integrity

A 2025 peer-reviewed multi-trade construction layout case study reported substantial gains for the studied project (including a reported 68% reduction in total labour hours and 18% fewer days, and rework falling from an estimated 6.42% to 0.25%). These are case-study results, not universal SOTA or an independently controlled multi-site comparison. The paper also documents an important counterexample: about one hour of robotic printing was placed incorrectly because a drywall CAD file used the wrong coordinate system; the model was realigned and the work reprinted. It also describes approximately five working days per floor for VDC file preparation and significant additional coordination effort.

Source: Kam et al., *Comparison of dusty robotics and traditional layout methods*, Construction Robotics (2025-09-15): https://link.springer.com/article/10.1007/s41693-025-00163-z

This changes the fair benchmark denominator. Measure total crew-hours and elapsed time per accepted feature / accepted printed metre, including source model preparation, multi-trade federation and revision coordination, coordinate/frame validation, survey control setup, exporter configuration, robot setup, operator/spotter, print time, QA, rework and recovery. Report robot-only print throughput separately, but never let it stand in for accepted project output.

The pre-print input-integrity gate is now tracked in [#3561 / PR #3564](https://github.com/Luminous-Dynamics/symtropy/pull/3564). It consumes #1607's survey and transform references and checks exact source/export revisions, frame bindings, independent checkpoint evidence and feature inventory before a layout manifest is admitted. It does not calculate transforms or establish survey truth.

This profile therefore treats three outcomes as separate:
1. correct input admission (model, revision, frame, transform, inventory);
2. physical output quality under the common independent referee;
3. total-workflow productivity per accepted output.

A robot can perform well on (2) for a wrong task input and still produce a failed project outcome. None of these three dimensions substitutes for the others.

