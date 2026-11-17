# Construction Field Layout Acceptance Benchmark v0.1

**Status:** proposed preregistration profile. No physical runs or vendor head-to-head measurements are claimed.  
**Purpose:** compare construction-layout systems by independently accepted site output, whole-workflow cost, repeatability and operational robustness.

## 1. Thesis: score accepted construction state, not robot motion

The benchmark's primary unit is a **Layout Acceptance Unit (LAU)**: one stable design-intent entity (or explicitly defined connected group) that has been independently measured against the frozen design and accepted under a predeclared tolerance and semantic rule.

Examples include a line segment, point, arc, text/string object, wall-stationing feature, opening/penetration mark or floor-elevation point. LAU families remain separate. A long line cannot be counted as equivalent to one point or one label merely because both are one CAD object; report geometric workload (length, features, complexity) alongside LAU count.

Each design ID must reconcile into exactly one current state:
- ACCEPTED: independently measured and satisfies the frozen predicate;
- REJECTED: independently measured and fails the predicate;
- HELD: execution or evidence is incomplete, stale or contradictory;
- WAIVED: explicitly accepted by an authorized project role with a traceable reason and scope;
- NOT_ATTEMPTED: not attempted during this run.

The robot's own "printed", "complete", station-verified, progress, confidence or accuracy flag is an observation only. It cannot establish acceptance.

The primary outcome is **independently accepted LAUs per total crew-hour**, reported alongside geometric residuals, first-pass yield, rework, elapsed duration, energy, material use and intervention burden. Never use a single composite score that can trade away a false negative, a missing design entity or a safety regression for higher throughput.

## 2. Published comparator envelope (not a league table)

The preregistration must freeze a dated source snapshot and exact configuration for every vendor run.

| System / study | Public reference | How to compare fairly |
|---|---|---|
| HP SitePrint | Current product page: ±2 mm layout accuracy tolerance and ±0.8 mm average floor-level accuracy under the specified high-accuracy setup: SMR prism, High Accuracy Print Mode, named 1-inch total stations and 5–30 m operating distance. Current page separately cites up to 30% productivity improvement over its prior software version from internal CAD-file tests; its stated MEP and interior-wall figures differ. | Reproduce the named prism, total station, mode and distance. Do not apply the high-accuracy footnote to other configurations. Do not treat internal software-version improvements or pilot "up to 10×" case-study statements as independent head-to-head throughput results. |
| Dusty FieldPrinter 2 | Current technical specification: ±1/16 in (±1.6 mm) print accuracy given accurate surveyed control; 0.75 m/s solid-line print speed; 0.50 m/s text/point speed; 1.0 m/s autonomous navigation with obstacle sensors enabled. Uses a Leica Absolute Laser Tracker and specified station/control-point procedure. | Record tracker model, stationing and control network, range, surface, environment, print mode, geometry type and firmware. Print-head speed is not daily or whole-project production rate. |
| Dusty November 3, 2026 release | Wall stationing beta; movable/offsettable obstacles; point/line-end improvements accounting for acceleration; distance-sensor firmware tuning intended to reduce false cliff stops. | Test these as new strata: surveyed-control vs wall stationing; static vs moved obstacles; constant speed vs start/stop endpoints; cliff detection false stops and missed edges. Record beta eligibility and exact firmware. |
| Independent multi-trade case study | Kam et al., *Construction Robotics* (2025): one eight-floor medical-office project reported 68% fewer total labor hours, 18% fewer layout days, 50% accuracy improvement, 99% less physically straining work, and rework from 6.42% to 0.25%. Project-wide multi-trade production was reported as 246 linear ft/hour, compared with 352 ft/hour in an earlier, more constrained case. | Strong field-workflow comparator, but single case study. The manual baseline used estimates/interviews rather than a simultaneously observed matched-floor control. Preserve this limitation when quoting it. |

Source register (recheck at each preregistration):
- HP SitePrint: https://www.hp.com/us-en/printers/site-print/layout-robot.html
- Dusty FieldPrinter specifications: https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs
- Dusty November 2026 release notes: https://support.dustyrobotics.com/hc/en-us/articles/58160105858587-November-2026-Release-Notes-Wall-Stationing-Beta-and-Movable-Obstacles
- Kam et al. 2025: https://link.springer.com/article/10.1007/s41693-025-00163-z

These are not identical statistical claims. A "tolerance", an average accuracy value, a speed ceiling and a retrospective project productivity estimate must never be compared as if they were the same statistic.

## 3. Two evaluation arms

### Arm A — vendor-configuration reproduction

Reproduce each public vendor condition exactly, including hardware, prism/tracker/total-station model, firmware, operating mode, control-point geometry, station range, environment, print content and output measurement procedure. Deviations are recorded and the run is a separate stratum, not a partial pass.

### Arm B — common independent referee

Each platform receives the same frozen design-intent package, site frame, task strata, obstacle/workcell layout, reference network, surface/environment strata and acceptance predicates. An independent calibrated reference instrument measures the as-built marks. The instrument and its reference chain must not be the robot's own localization/control source. Where a common instrument cannot cover all devices, use a calibrated transfer artifact and crossed instrument/lab checks; preserve reference-chain uncertainty.

The evaluator is blind to vendor identity when scoring geometry and entity acceptance where practical. Operator safety cannot be blinded. Freeze the evaluator and scorer builds before the campaign.

## 4. Independent metrology and uncertainty budget

The measured mark residual is not the robot's displayed error. It is the difference between the frozen intended geometry in the declared site frame and the independently measured deposited/constructed mark.

At minimum record and separately estimate:
- reference-instrument calibration and scale/axis uncertainty;
- control-network adjustment uncertainty and station setup;
- instrument centring, reflector/prism, tracking and line-of-sight effects;
- frame registration and transform uncertainty;
- design/model extraction and entity correspondence;
- substrate movement, temperature and surface effects;
- mark-width/edge extraction uncertainty and scorer repeatability;
- robot control/localization/tracking uncertainty, as a separate system channel rather than a correction that hides observed residuals.

For each reported measurement provide residual, reference uncertainty and traceable calibration evidence. Use a covariance-aware uncertainty propagation where dependencies are known. Do not add correlated terms as if independent. Report signed bias and the full residual distribution; never report only an average.

**Resolution gate:** for a proposed 1.0 mm p95 claim, the independent reference method must demonstrate expanded uncertainty U95 no greater than 0.25 mm on the relevant axis/geometry family. If the reference chain cannot resolve the claim, mark the result NOT_TESTABLE at that resolution. Do not award a pass or infer an advantage.

Measure XY and Z separately. For lines, measure orthogonal centerline residual and endpoints/start-stop overshoot separately. For arcs, measure radial/normal residual and angular coverage. For points, measure 2D/3D position. For text, measure anchor position, orientation, glyph/line legibility and association with the correct design ID. Report ink/mark width separately from centerline placement.

## 5. Frozen task families and adversarial strata

Run each applicable system on the same family-specific corpus:

1. **Simple layout:** long straight lines, orthogonal lines, grids, points, arcs, text and dimension labels.
2. **Dense MEP:** small points, dense annotations, penetrations, hangers, sleeves and closely spaced parallel lines.
3. **Multi-trade coordination:** overlapping structural, architectural, mechanical, electrical, plumbing and fire-protection layers; explicit clash/precedence and source design IDs.
4. **Short segments and endpoints:** many short lines, corners, line starts/stops, acceleration/deceleration and point placement.
5. **Obstacles:** columns, walls, temporary materials and irregular objects; static obstacles and obstacles moved after initial stationing.
6. **Stationing:** surveyed control configurations, varying point geometry/range, station relocation, wall stationing beta and deliberate frame/transform discrepancies.
7. **Occlusion and tracking:** partial/temporary loss of tracker line of sight, blind zones, reflected/ambiguous surfaces, tracker relocation and recovered tracking.
8. **Surface/environment:** uneven slab, slope, dust, dampness, low/high temperature within declared equipment operating envelope, lighting/reflections, and surface discontinuities.
9. **Floor elevation:** point elevation and grid/heatmap measurement as a separate Z-metrology task with its own reference.
10. **Partial execution/restart:** interruption after partial output; restart without duplicate accepted LAUs, lost WIP or silently skipped entities.
11. **Multiple robots:** crossing routes, shared work zones, overlapping task schedules, stale obstacle state and independent clock domains. Start with instrumented fixtures and empty exclusion zones; no human exposure is needed to validate collision logic.
12. **Configuration changes:** tool/payload/print head, battery, firmware, sensor profile, collision geometry, calibration, frame, design revision and clock mapping changed or expired between stages.

For every stratum, include negative controls and injected faults: wrong frame, stale stationing, moved obstacle not propagated, missing entity, duplicate ID, dropped event, out-of-order completion, sensor dropout, timestamp reset, wrong tool offset and misleading robot self-report.

## 6. Whole-workflow time and resource accounting

Record wall-clock elapsed time and human/equipment time separately. Account for all work from design package receipt through independently accepted output:
- BIM/VDC cleanup, coordinate reconciliation and trade coordination;
- import/export, task compilation and revision review;
- site preparation, survey/control installation and stationing;
- calibration, station verification and robot setup;
- route planning, travel, printing and measurement;
- battery/ink/material handling;
- inspection, data synchronization and acceptance review;
- interruptions, waiting for other trades, weather, recovery and operator interventions;
- rejected output, rework, re-stationing, and decommission/changeover.

Report crew-hours by role as well as total hours. An unattended machine does not imply zero labor if it requires supervision, setup, recovery or back-office correction. Distinguish robot operational hours, operator-on-site hours and the time of all supporting roles.

Primary productivity:
\`accepted_LAUs / total_crew_hours\`

Also report accepted line metres/hour by line family, accepted points/hour, accepted arc metres/hour and accepted text entities/hour. These task-family metrics must not be collapsed into one raw linear-foot figure.

Rework ratio:
\`rework_crew_hours / (first_pass_crew_hours + rework_crew_hours)\`

First-pass yield:
\`first_pass_accepted_LAUs / attempted_LAUs\`

Coverage:
\`(accepted + rejected + held + waived + not_attempted design IDs) / frozen design IDs\`
must equal 100% after reconciliation; the categories must be mutually exclusive at the reporting time.

## 7. Preregistered stretch targets (goals, not achieved results)

Freeze thresholds before observing outcomes. Where the reference uncertainty is insufficient, mark the threshold untestable.

| Dimension | Proposed stretch target | Gate / reporting rule |
|---|---|---|
| Layout geometry | p95 absolute XY residual ≤1.0 mm; p99 ≤2.0 mm on the declared precision profile | U95 ≤0.25 mm; report axis-specific signed bias, maximum, family-specific distributions and reference uncertainty. Also reproduce vendor-stated conditions separately. |
| Z / floor elevation | Establish a separate task-specific tolerance and reference capability | Do not borrow the XY layout threshold or compare grid-average elevation to single-point height. |
| Entity completeness | 100% reconciliation of design IDs; zero critical omitted or duplicated entities | Every entity is ACCEPTED, REJECTED, HELD, WAIVED or NOT_ATTEMPTED. Unknowns are never counted as accepted. |
| First-pass quality | ≥99.5% on the predeclared simple-layout profile | Report denominator, confidence interval, family and all exclusions. Complex MEP/multi-trade profiles remain separate. |
| Rework | ≤0.5% of accepted simple-profile LAUs requiring rework | Report both the entity rate and rework crew-hours, cause and responsible stage. |
| Whole-workflow productivity | At least 20% fewer crew-hours per accepted LAU than a matched manual baseline; stretch ≥30% over the best matched robotic baseline | No quality, completeness, safety or intervention regression. Count coordination, stationing, waiting, inspection and rework. |
| Repeatability | Independent second lab reproduces acceptance and residual distributions within the preregistered uncertainty model | New operator and separately calibrated reference chain. Lab/instrument common-mode dependencies are disclosed. |
| Robustness | Zero silent completion or accepted-state mutations after stale/missing/contradictory evidence | Injected stale frame, calibration, clock, sensor, tool and event faults must hold/reconcile future work. Historical acceptance remains immutable. |
| Human factors | Reduce high-strain work and exposure without increasing unprotected hazard | Record operator interventions, exclusion-zone incursions, ergonomic exposure, false stops and missed detections. A reduction in stops alone is not improvement. |

For any safety-related detection stratum, report observed false negatives and a confidence bound; "zero observed" is not "zero risk". For example, zero misses in 3,000 independent representative challenge trials only bounds the underlying miss probability to roughly 0.1% at one-sided 95% confidence under the assumed independent Bernoulli model. Clustered/common-cause trials require a different model and larger campaign.

## 8. Statistical design

- Use at least three independent labs as an initial transferability target; final lab, serial-unit, operator, session and case counts require pilot variance and a predeclared minimum practically important difference.
- Include more than one serial unit per platform where available and multiple operators/sessions per lab.
- Randomize task order and blind scoring where practical. Freeze code, model files, evaluator, acceptance rule and analysis before data collection.
- Separate platform, lab, unit, operator, session, task-family and environment effects using a crossed/nested design. If factors are confounded, explicitly say so rather than estimating unidentifiable variance components.
- Report effect sizes and confidence intervals, not only p-values. Use appropriate hierarchical models for continuous residuals and separate binomial/count models for acceptance, omissions, interventions and incidents.
- Keep all attempts, failures, exclusions and unresolved cases in a flow diagram. Never remove a failed run from the denominator without a preregistered exclusion reason.

## 9. Multi-robot and event-sourced site-state track

This is the next paradigm shift beyond autonomous marking.

Compile frozen design intent into an immutable work package containing entity IDs, source revision, coordinate frames/transforms, tolerances, dependencies, prerequisites and acceptance predicates. Execution emits observations. An independent referee measures. A separate site-state ledger changes an entity to ACCEPTED only after a valid acceptance receipt. Work can be resumed from that ledger after interruptions.

Required challenge cases:
- two robots with independent monotonic clocks and bounded, expiring clock maps;
- robot A reports complete while independent inspection rejects one entity;
- task interruption with partial work and restart;
- design or transform revision after some entities are accepted;
- a moved obstacle not reflected in one robot's local state;
- duplicate, delayed, missing or out-of-order events;
- two robots scheduled for intersecting space-time volumes;
- one robot's clock map expires while its physical work is still historically valid.

Historical accepted output remains historical. Future tasks using stale or contradictory configuration are held until requalified. An observation, broad-phase candidate or planner result must never authorize actuation.

## 10. Standards and applicability

Use these as references for a documented applicability assessment; do not claim conformity merely because the benchmark borrows their concepts:
- AMRA-201:2026, mobile robot general requirements and test methods for solid travel surfaces: https://amr-alliance.org/amra-201/
- ISO 10218-1:2025, industrial robots: https://www.iso.org/standard/73933.html
- ISO 10218-2:2025, industrial robot applications and robot cells: https://www.iso.org/standard/73934.html
- ISO 12100:2010, machinery risk assessment and risk reduction (confirmed current after its 2022 review): https://www.iso.org/standard/51528.html
- ASTM F45 building blocks: F3327 configuration recording, F3218 environmental conditions, F3381 stationary obstacles and F3244 defined-area navigation: https://www.astm.org/committee-f45
- NIST Mobility Performance of Robotic Systems, including metrology, artifacts, tracking, adaptability, task efficiency, quality and assurance: https://www.nist.gov/programs-projects/mobility-performance-robotic-systems

A competent safety assessor must determine the actual robot/system/site obligations, applicable national law, risk assessment and required validation. This benchmark does not replace those processes.

## 11. Reporting and claim discipline

Publish:
- frozen corpus and all case-family descriptors;
- exact platform/build/firmware/configuration manifest;
- measurement and uncertainty budget;
- raw per-entity observation receipts and independent referee data;
- reconciled entity-state ledger;
- complete event/intervention/rework timeline;
- analysis code and environment lock;
- per-family distributions, intervals and excluded/unknown counts;
- deviations from preregistration and all failed runs.

Use three claim levels:
1. **Software/reference verified:** schema, analytic tests, replay and software candidate-superset checks only.
2. **Laboratory measured:** independently instrumented fixtures with stated uncertainty and exact test configuration.
3. **Field replicated:** independently reproduced across sites/labs/operators with whole-workflow accounting.

Do not use level 1 evidence to claim level 2 or 3. Do not describe vendor brochure values, one-host microbenchmarks or synthetic trajectories as field SOTA. The benchmark succeeds when its protocol makes an unfair comparison difficult and a genuine improvement independently reproducible.
