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

## Evidence and ownership

- #3505/#3506 own physical validation protocol and ODD/metrology framework.
- #3521/#3522 own trial receipt schema and semantic admissibility.
- #1607 owns survey provenance, coordinate frame and uncertainty semantics.
- #1595 owns shared scenario identities and fault schedules.
- #3317/#3499/#2672 own snapshot, temporal and mission safety evidence respectively.

Every physical run should produce a #3522 trial receipt bound to the design revision, robot configuration, site snapshot, ODD, survey artifacts, reference instruments and independent adjudication.

## Non-goals

No universal robot ranking, no cross-task composite, no use of a vendor's own positioning output as independent ground truth, no inference of safety from positioning accuracy, and no claim of standards conformity or certification.
