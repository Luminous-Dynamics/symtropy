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

## 1.1. Source-model interoperability and pre-deployment correctness

The upstream input is not "a CAD file that the robot happens to understand." Freeze the source model, schema/format version, model revision, unit system, project coordinate reference, site frame, transform, trade-layer scope and information requirements. The compiler emits a vendor-neutral work package with stable design entity IDs, geometry, semantic type, tolerances, dependencies, prerequisites and acceptance predicates.

For IFC-based projects, use the official IFC 4.3.2.0 / ISO 16739-1:2024 exchange schema and IDS 1.0 information requirements as the open source-model contract. Validate IFC schema plus project IDS rules before task compilation. Preserve each IFC GlobalId and source revision in the downstream entity manifest. A valid IFC file can still contain a wrong transform, incorrect units, stale revision, wrong property values or unresolved coordination issue; schema validity is necessary but not sufficient.

Add a **pre-deployment model challenge track** before any physical robot operation:
- wrong translation, rotation, scale, axis convention, unit conversion and vertical datum;
- site-vs-model coordinate transform swapped, missing or stale;
- duplicate/missing/changed entity IDs or source GUIDs;
- outdated model revision, trade layer or linked model;
- objects missing required properties or violating IDS requirements;
- conflicting penetrations, walls, equipment bases and work sequencing;
- a previously accepted package whose source revision has been superseded.

Measure detection recall, false alarms, diagnosis time and the rate of unsafe packages correctly held before dispatch. The acceptance target is zero dispatch from an unresolved source identity/frame/units/revision contradiction—not merely a warning in a report. This directly addresses a field-study finding that inaccurate source information uploaded to a layout robot can produce inaccurate physical layout, and its recommendation for automatic coordinate-system checks.

## 2. Published comparator envelope (not a league table)

The preregistration must freeze a dated source snapshot and exact configuration for every vendor run.

| System / study | Public reference | How to compare fairly |
|---|---|---|
| HP SitePrint | The product page states ±2 mm layout tolerance and ±0.8 mm average floor-level tolerance only for its high-accuracy setup: SMR prism, High Accuracy Print Mode, tested 1-inch total stations and 5–30 m range. The March 2026 datasheet also gives a standard 3-inch total-station condition of ±3 mm average layout tolerance and ±2 mm average floor-level tolerance. The page headline advertises "up to 10×" productivity gains, but its footnote quantifies up to 30% versus the previous VP2.0 software using internal CAD-file tests (MEP >30%, interior-wall layouts 8%). | Benchmark standard and high-accuracy configurations separately; record the exact total station, prism, mode and distance. Treat the 10× headline as a marketing claim until its baseline and scope are disclosed. The 30%/MEP/wall figures are software-version internal tests, not independent full-workflow comparisons to manual crews or competing robots. |
| Dusty FieldPrinter 2 | Current technical specification: ±1/16 in (±1.6 mm) print accuracy given accurate surveyed control; 0.75 m/s solid-line speed; 0.50 m/s text/point speed; 1.0 m/s navigation with obstacle sensors on. Positioning uses a Leica Absolute Laser Tracker and at least 3 non-collinear visible control points per station. The platform is listed as IP51-equivalent (not rain-safe); cliff sensors have a stated trigger threshold and finite field of view. | Reproduce the exact tracker, control network, station range, operating surface, weather, print mode and geometry. Measure line endpoints/points separately from continuous-line speed. Include narrow gaps, openings, slab edges, false stops and missed-edge controls; do not interpret a lower false-stop rate as safer without false-negative measurement. |
| Dusty November 3, 2026 release | Wall-stationing beta requires at least 2 non-parallel walls and 3 or more well-spaced scans per wall; operators can move/offset custom obstacles and sync edits after reconnecting; station verification adds 1/32 in, 1/16 in and 1/8 in choices; firmware 1.6 tuning reduces false cliff stops; acceleration compensation improves points and line ends. | Treat wall stationing as a separate beta stratum and record scan geometry. Test obstacle edits while connected and offline/reconnected. For cliff tuning, preregister both false-stop and missed-edge rates. For endpoint updates, isolate acceleration/deceleration residuals. A feature release is not an independent accuracy or safety result. |
| CivDot+ / CivDot | Civ Robotics currently advertises CivDot+ point marking at "3/100 ft (8 mm)" and up to 3 points/minute; CivDot spray marking at 1/10 ft (30 mm) and up to 8 points/minute. The same official page describes GNSS/RTK, IMU and robotic laser-pointer modes. | Outdoor civil, solar, piling and earthworks point-layout track only. The published CivDot+ conversion is internally inconsistent: 3/100 ft equals 9.144 mm, not 8 mm. Preserve the exact vendor wording and obtain clarification before making a numerical comparison. Do not compare point/minute throughput to interior continuous line printing. |
| CivDot Mini | Civ Robotics advertises sub-inch (25 mm) spray marking and up to 200 ft (60 m) of solid/dashed lines per minute for roads, airports, parking and curb/gutter marking. | Outdoor linear-marking track. Independently measure paint centerline, line width, substrate/surface and geospatial reference. Its nominal linear speed is not directly comparable to an interior layout robot's complete BIM-to-accepted-output rate. |
| Rugged Robotics Mark1 | Rugged advertises under 1/8 inch accuracy, a 5.5-inch printhead, multi-trade single-pass layout and operation using passive targets without a total station. | Treat as a vendor-published claim until the exact target network, reference method, geometry, coverage and field conditions are disclosed. Compare on the common interior/multi-trade arm and a separate passive-target stationing arm. |
| Hilti Jaibot (adjacent execution system) | Hilti describes a BIM-driven semi-autonomous ceiling drilling robot positioned using a total station, with optional path scanning for obstacles and progress reporting. | Not a floor-layout printer and must not be ranked in the mark-placement table. Include as a downstream construction-execution benchmark: independently accepted hole location/depth/diameter, task completion, crew-hours and rework. |
| Independent multi-trade case study | Kam et al., *Construction Robotics* (2025): one eight-floor medical-office project reported 68% fewer total labor hours, 18% fewer layout days, 50% accuracy improvement, 99% less physically straining work, and rework from 6.42% to 0.25%. Project-wide multi-trade production was reported as 246 linear ft/hour, compared with 352 ft/hour in an earlier, more constrained case. | Strong field-workflow comparator, but single case study. The manual baseline used estimates/interviews rather than a simultaneously observed matched-floor control. Preserve this limitation when quoting it. |

Source register (recheck at each preregistration):
- HP SitePrint: https://www.hp.com/us-en/printers/site-print/layout-robot.html
- HP SitePrint March 2026 datasheet: https://h20195.www2.hp.com/v2/getpdf.aspx/c08651741.pdf
- Dusty FieldPrinter specifications: https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs
- Civ Robotics layout robots / CivDot+ / CivDot / CivDot Mini: https://www.civrobotics.com/robots/layout-robots
- Civ Robotics FAQ on accuracy and RTK behaviour: https://www.civrobotics.com/faq
- Rugged Robotics Mark1 claims: https://rugged-robotics.com/about/meet-rugged/
- Hilti Jaibot overview: https://www.hilti.co.za/content/hilti/META/ZA/en/business/business/trends/jaibot-semi-autonomous-drilling-robot.html
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

1. **Simple interior layout:** long straight lines, orthogonal lines, grids, points, arcs, text and dimension labels.
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
13. **Outdoor civil staking:** point-density and point-spacing distributions, GNSS/RTK fix quality, base station/NTRIP mode, slope, terrain, line-of-sight and stake/paint marking method. Report points/minute and accepted coordinate residual separately.
14. **Outdoor linear marking:** roads, airport/parking and curb/gutter lines with long straight and curved runs, dashed geometry, line-width and centerline error, surface texture, GNSS/RTK environment and weather. Report continuous marking speed separately from accepted coverage.
15. **Downstream drilling:** ceiling/MEP hole center, depth, diameter, pattern completeness, obstacle/path scan and material-specific quality. This is a separate task family for systems such as Hilti Jaibot, not a layout-printing accuracy comparison.

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
| Whole-workflow productivity | Gate: ≥20% fewer total crew-hours per accepted LAU than a matched manual baseline. SOTA challenge: ≥70% fewer whole-workflow crew-hours than a matched observed manual baseline, intended to exceed the 68% reduction reported in the 2025 single-project case study. Stretch against industry robotics: ≥30% higher accepted output/crew-hour than the best matched robotic baseline. | The 68% study's manual baseline was estimated/interview-based, not a simultaneous matched control. Our 70% is a prospective target, not a current result. Count model coordination, VDC, stationing, waiting, supervision, inspection, recovery and rework. Require quality/safety non-inferiority. |
| Schedule | ≥20% fewer elapsed layout days than a matched manual workflow | Challenge target is just above the 18% reduction reported in the 2025 case study. Compare same scope, site sequencing and work windows; report weather/trade congestion and calendar-time confounders. |
| Rework | ≤0.5% of accepted simple-profile LAUs require rework; stretch ≤0.1% | Report entity rework rate and rework crew-hours. To compare with published case-study percentages, recalculate from raw data using the same numerator and denominator; do not compare unlike definitions. |
| Repeatability | Independent second and third labs reproduce acceptance and residual distributions within the preregistered uncertainty model | New operators, serial units and separately calibrated reference chains. Lab/instrument common-mode dependencies are disclosed. |
| Robustness | Zero silent completion or accepted-state mutations after stale/missing/contradictory evidence | Injected stale frame, calibration, clock, sensor, tool, revision and event faults must hold/reconcile future work. Historical acceptance remains immutable. |
| Human factors | ≥99% reduction in physically straining worker-minutes versus matched manual workflow, with no safety regression | This targets the magnitude reported in the 2025 case study using a directly observed control. Also record operator interventions, exclusion-zone incursions, ergonomic exposure, false stops and missed detections. A reduction in stops alone is not improvement. |

For layout geometry, remember that a vendor-stated tolerance (HP ±2 mm in its high-accuracy configuration or Dusty ±1.6 mm under surveyed control) is not automatically a p95 statistic. Our p95 ≤1.0 mm / p99 ≤2.0 mm target is scored from a measured distribution with an explicit uncertainty budget. Claim superiority only after reproducing the vendor's stated configuration on the same blinded geometry corpus and applying the same referee and distribution estimator.

For any safety-related detection stratum, report observed false negatives and a confidence bound; "zero observed" is not "zero risk". For example, zero misses in 3,000 independent representative challenge trials only bounds the underlying miss probability to roughly 0.1% at one-sided 95% confidence under the assumed independent Bernoulli model. Clustered/common-cause trials require a different model and larger campaign.

## 8. Statistical design

- Use at least three independent labs as an initial transferability target; final lab, serial-unit, operator, session and case counts require pilot variance and a predeclared minimum practically important difference.
- Include more than one serial unit per platform where available and multiple operators/sessions per lab.
- For the manual comparator, prefer a within-project randomized blocked trial: match zones/floors by area, trade mix, layout density, obstacle congestion and substrate; randomize eligible blocks to robot or manual workflow; use a cross-over/stepped-wedge design when operationally feasible. Directly observe the manual crew and include its normal survey/coordination/inspection effort. Do not use retrospective crew estimates as the sole baseline.
- If randomization is infeasible, use matched projects or matched floor/zone pairs with preregistered covariate adjustment for crew experience, model maturity, trade count, weather, labor availability and sequencing. Report residual confounding; don't present adjusted estimates as randomized evidence.
- Randomize task order and blind geometry scoring where practical. Freeze code, model files, evaluator, acceptance rule and analysis before data collection.
- Separate platform, lab, unit, operator, session, task-family, project, zone/floor and environment effects using a crossed/nested design. Cluster confidence intervals at the project/zone level to avoid treating every line segment as an independent project replication.
- Report effect sizes and confidence intervals, not only p-values. Use hierarchical models for residuals and separate binomial/count models for acceptance, omissions, interventions and incidents. Preregister minimum practically important differences and stop/exclusion rules.
- Keep all attempts, failures, exclusions and unresolved cases in a flow diagram. Never remove a failed run from the denominator without a preregistered exclusion reason.

## 9. Multi-robot and event-sourced site-state track

This is the next paradigm shift beyond autonomous marking.

Compile frozen design intent into an immutable work package containing entity IDs, source revision, IFC GlobalIds where available, coordinate frames/transforms, tolerances, dependencies, prerequisites and acceptance predicates. The robot executes only a package bound to a qualified station/frame. Execution emits robot observations; a separate referee measurement is scored independently. An append-only site-state ledger changes an entity to ACCEPTED only after the scorer validates a referee receipt bound to the exact package/revision/frame/transform/profile. Work can be resumed from that ledger after interruptions.

The first Rust reducer is now implemented in src/site_state_ledger.rs. It role-separates design authority, surveyor, robot, referee and site-quality authority; binds robot observations to the robot that began the attempt; rejects stale station transforms and broken event chains; supports idempotent replay; preserves historical acceptance when a new package supersedes the old one; and keeps unresolved measurements HELD. The caller still has to verify canonical event hashes and signatures. This is a deterministic single-ledger projection, not yet a distributed consensus or offline multi-writer merge protocol.

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

Use these as a clause-by-clause applicability map, not as a claim that this benchmark or a prototype conforms to them:

- **AMRA-201:2026** (published 26 July 2026) specifies general requirements and test methods for mobile robots operating on solid travel surfaces, including charging stations. It is a useful mobile-platform test-method baseline, not a construction-layout workflow certification: https://amr-alliance.org/amra-201/
- **ISO 10218-1:2025** covers industrial robot machinery; **ISO 10218-2:2025** covers industrial robot applications and cells. Determine whether the actual construction layout system, its robot and its integration fall within each part's scope; do not use the industrial-manipulator standards alone to close mobile-base hazards: https://www.iso.org/standard/73933.html and https://www.iso.org/standard/73934.html
- **ISO 3691-4:2023** is the current published reference for driverless industrial trucks and their systems (including qualifying AGVs/AMRs). Screen the machine definition and operating environment before applying it. ISO/DIS 3691-4, a proposed third edition, is in development and is not yet the published replacement: https://www.iso.org/standard/83545.html and https://www.iso.org/standard/88615.html
- **ISO 12100:2010** provides machinery risk-assessment and risk-reduction methodology; ISO says it was confirmed current after its 2022 review, while ISO/DIS 12100.3 is now under development. Recheck status at each design freeze: https://www.iso.org/standard/51528.html
- **ASTM F45** supplies useful A-UGV method components: F3327 configuration recording, F3218 environmental conditions, F3381 stationary obstacles, F3244 defined-area navigation and F3243 communications impairments: https://www.astm.org/committee-f45
- **NIST Mobility Performance of Robotic Systems (MPRS)** is an ongoing measurement-science programme for mobility-system test methods, artifacts and performance assessment; its current page was updated 24 April 2026: https://www.nist.gov/programs-projects/mobility-performance-robotic-systems
- **IFC 4.3.2.0 / ISO 16739-1:2024** is the current official IFC schema release for exchanging built-asset information. **buildingSMART IDS 1.0** is an official computer-interpretable standard for specifying and automatically checking IFC information requirements. Use them as the source-model and preflight interchange layer, then compile a frozen robot-neutral manifest rather than sending vendor-specific CAD as the only source of truth: https://www.buildingsmart.org/standards/bsi-standards/industry-foundation--classes/ ; https://www.iso.org/standard/84123.html ; https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/

A competent safety assessor must determine the actual robot/platform/system/site obligations, applicable national law, machinery classification, risk assessment and required validation. In particular, a generic mobile-robot standard, a manipulator standard, and a driverless-truck standard have different scopes. This benchmark does not replace those processes.

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
