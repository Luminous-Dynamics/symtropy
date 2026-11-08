# Panel Disassembly and Repair Benchmark Protocol v0.1

**Parent:** PANEL-DISASSEMBLY-001 #4150  
**Lifecycle parent:** BUILT-ASSET-ROBOT-LOOP-001 #4043  
**New-build comparator:** ABOVEGROUND-PANELCELL-001 #3773  
**Status:** experimental protocol; synthetic and laboratory stages are not building certification or proof of field productivity.

## 1. Why this benchmark exists

New-build construction robotics often optimizes installation, placement or initial fabrication. City systems also need to be inspected, maintained, adapted, repaired and eventually disassembled. A building component is not circular merely because it can be separated; its identity and condition must survive removal, an applicable requalification route must be passed, and it must still satisfy the declared function after reassembly.

The test subject is a **controlled, non-load-bearing envelope/service panel** with replaceable trim, gasket, service insert and connection hardware. It is deliberately bounded to a guarded laboratory setting. It is not a structural wall, fire-resisting assembly, façade-at-height or occupied-building trial.

The test compares four work methods on the same frozen task and acceptance basis:

- **R0 — Manual specialist:** documented hand tools and a qualified operator.
- **R1 — Instrumented manual:** same workflow with calibrated metrology, digital instructions and part identification.
- **R2 — Teleoperated / assisted:** robot presents, holds or manipulates a part under human task authority.
- **R3 — Guarded automation:** a fixed, risk-assessed cell executes only a validated, repeatable sub-sequence.

An alternative connection design may be compared as a separate design arm, but only if it meets the same functional requirements and the exact differences in product design are disclosed.

## 2. Research and standards crosswalk

### ISO 20887:2020

ISO 20887:2020, *Sustainability in buildings and civil engineering works — Design for disassembly and adaptability*, is current: ISO reports that it was reviewed and confirmed in 2025. It applies to buildings, civil works and constituent parts, across new construction, renovation and refurbishment. It gives principles and guidance for design for disassembly/adaptability and guidance on measuring performance, but **does not set universal numerical performance levels**. Therefore this protocol must derive task-specific acceptance criteria from the article, functional requirement and independent test method; it must not claim an ISO-derived universal “disassembly pass score”.

Official ISO record: https://www.iso.org/standard/69370.html

### Disassembly Ease Index (2025)

The 2025 *Disassembly Ease Index* work identifies 17 variables, 12 factors and two ratios for evaluating building disassembly. Its application to a structural system highlighted the importance of connection types. It is a design-method comparator. It is not a robot-performance metric and not a qualification of the panel article.

DOI: https://doi.org/10.1016/j.resconrec.2025.108215

### Automated deconstruction of concrete building parts (2025)

Heuer et al. report a semi-automated concrete-part deconstruction system for reuse. The publisher abstract reports an approximately 83% reduction in setup/disassembly time compared with manual methods and 1.5005 mm end-effector positioning accuracy. Those are reported results for that system and concrete task, **not a target transferable to our panel** without reproducing the article, boundary, test and acceptance basis.

DOI: https://doi.org/10.1016/j.jobe.2025.113952

### Human–robot collaboration in building disassembly (2026)

Slezák et al. (published 6 July 2026) propose a multi-agent human–robot architecture using domain retrieval, physics simulation and human negotiation. The article tests three progressively complex scenarios (spatial adaptation, decision-making and learning). It explicitly states that current available technology does not enable full automation. This is a proof of concept for adaptive HRC—not evidence that a general autonomous deconstruction system is ready for deployment.

DOI: https://doi.org/10.1007/s41693-026-00189-x

### Implication for CityMaker

Our differentiation should be evidence-qualified lifecycle performance, not an assertion that a robot can “build or dismantle anything”. We should compare:
- connection and part design for inspectability/serviceability;
- component-level damage and recovery yield;
- functional requalification and post-reassembly testing;
- total labour and operator burden;
- safe abstention and safe recovery;
- repeated-cycle durability and maintenance burden.

## 3. Article and functional basis

Freeze the following before any candidate result is evaluated:

1. Panel article ID, exact geometry and design revision.
2. BOM, component/lot identities, material and condition specifications.
3. Connection graph, fastener family, tool access and declared assembly/removal order.
4. As-built reference geometry and independent measurement method.
5. Functional requirements and acceptance predicates.
6. Workcell/fixture, robot and tool profile; calibration and sensor revisions.
7. Safe operating envelope, isolation procedure, exclusion zones and recovery route.
8. Test procedure, trial order, sample-size rationale and analysis plan.

Do not use geometry resemblance as proof of structural, fire, water, air, acoustic or durability performance. The first article is non-load-bearing; the relevant function test for each replaceable panel layer must still be independently specified.

## 4. Repeated-cycle protocol

For each panel/component cycle, record these distinct transitions:

`assembled -> observed -> isolated -> identified -> removal-authorized -> removed -> condition-assessed -> repair/requalify -> reassembled -> independently-tested`

A component may end in `accepted_reusable`, `repair_required`, `rejected`, `unknown` or `not_inspected`. These are not synonyms.

Required evidence:
- before-removal component and connection identity;
- observation coverage, localization and uncertainty;
- fastener/adhesive/connection type and condition;
- validated removal tool and sequence;
- any temporary support / fixture requirement;
- removed-part custody and measured condition;
- damage to host and adjacent components;
- repair process, replacement items and material lot identities;
- requalification test criteria and independent results;
- reassembly procedure and measured fastening evidence;
- final independent function test;
- time, labour, energy, consumables, waste, rework and recovery events.

**A successful removal is not a successful repair. A successful repair is not a requalified component. A requalified component is not an accepted assembly until the post-reassembly function test passes.**

## 5. Synthetic conformance cases

Freeze at least 24 known-answer cases:

1. Visible fastener, correct identity, correct tool and declared order.
2. Fastener hidden by occlusion; abstain until further observation.
3. Incorrect driver/bit profile; block.
4. As-built fastener identity differs from the drawing revision.
5. Stripped fastener head; only an explicitly qualified alternate process is admissible.
6. Corrosion condition exceeds the declared removal/damage threshold.
7. Bonded connection has no qualified release route; do not label it reversible.
8. Connection dependency/order is stale or absent.
9. Removal violates declared fixture/stability/exclusion conditions.
10. Temporary support required but no authorized support profile exists.
11. Part ID does not match the released BOM.
12. Coordinate frame or registration is stale/uncertain.
13. Observation occludes a connection required by the procedure.
14. Removal damages an adjacent component.
15. Removed part fails independent dimensional requalification.
16. Apparently intact part has unknown material/condition evidence.
17. Single-use fastener is incorrectly proposed for reuse.
18. Recovered component needs cleaning or repair before requalification.
19. Repaired component passes the declared independent function test.
20. Repaired component fails the function test and remains rejected.
21. Reassembly fastening evidence is missing or outside the declared range.
22. Geometry passes but required water/air/acoustic test is absent; remain Unknown.
23. Repeated cycles exceed the declared wear/damage limit.
24. A shared component/tool/resource lot is double-counted across two panels.

All numerical values in fixtures are synthetic. A test case must not silently imply a real fastener torque, panel load rating, material grade or safety limit.

## 6. Fair trial design

Compare R0–R3 on the same frozen panel article, material lots, defect states, tool requirements, work content and independent acceptance predicates.

- Randomize or block trial order where practical to control operator learning, fatigue, material order and setup effects.
- Preregister the primary operational outcome and analysis before inspecting results.
- Establish the manual R0 baseline before setting a productivity-improvement target.
- Estimate variance in a pilot, then determine sample size from a declared minimum detectable effect and expected clustering by article/operator/lot.
- Keep failed, damaged, reworked, unresolved and rejected components in the denominator.
- Use independent inspection and functional testing not controlled by the robot or candidate planner.
- Report all role-specific human time: setup, kitting, supervision, tool changes, safe holds, cleaning, inspection, repair, recovery and training.
- Distinguish synthetic, simulated, recorded replay, instrumented laboratory and field observations.

No one may infer a physical emergency stop or safe state from a simulation label. Physical operation requires separate risk assessment, guarding/interlocks, commissioning and accountable authorization.

## 7. Metrics

Report dimensions separately:

| Dimension | Measures |
|---|---|
| Access and identification | coverage, connection-ID accuracy, unknown/occluded fraction, wrong-revision rate |
| Removal | first-pass removal acceptance, removal time, tool changes, exceptions, host/adjacent damage |
| Recovery | mass and component count removed, recoverable, requalifiable, requalified and reusable—kept distinct |
| Repair and requalification | repair time, requalification pass rate, rejection causes, replacement parts and residual uncertainty |
| Reassembly | fastening/connection evidence, geometry, reassembly time, repeated-cycle wear |
| Function | independent post-reassembly tests, recurrence and accepted service state |
| Productivity | accepted repaired assemblies per total workflow labour-hour; p50/p90 times for each stage |
| Human factors | exposure removed and new exposure introduced, physical burden, cognitive burden, task interruption and role distribution |
| Lifecycle | consumables, waste, measured component reuse, replacement demand and declared lifecycle boundary |
| Robustness | response to missing/stale identity, changed calibration, damaged fastener, sensor fault and unexpected condition |

A kilogram removed is not a kilogram reused. A part reinstalled is not an accepted reusable component until requalified. A reduction in robot cycle time is not a productivity gain if total labour or damage increases.

## 8. Proposed evidence ladder and goals

**E1 — software conformance**
- 100% agreement with an independent oracle on the frozen finite corpus.
- Zero removal approval on unknown/stale/wrong-revision connection data.
- Zero reuse without independent component requalification.
- Zero double counting of parts, lots or shared tools/resources.
- Deterministic, exact-head receipts for article, design, BOM, tool, calibration, procedure and corpus.

**E2 — reference-method comparison**
- Compare connection-design indicators with ISO 20887 principles and the published Disassembly Ease Index.
- Compare applicable manipulation/fastener tasks with NIST assembly measures where their task definitions are matched.
- Report deviations; do not translate a small-parts test-board result into building-panel performance.

**E3 — guarded laboratory article**
- Matched R0–R3 trials with independent metrology and function testing.
- Multiple installation/removal cycles with damage and requalification after each.
- Establish a trustworthy manual baseline before setting numeric productivity targets.
- No accepted output with a known critical functional failure.

**E4 — bounded, supervised field pilot**
- Only after lab results and task-specific engineering/safety approval.
- Controlled, non-occupied or access-controlled article/asset; property and worker permissions; professional review.
- Report site variance, access, logistics, service interruption and recovery.

**E5 — replication**
- Repeat across a second article/material/connection family and a separate team/site.
- Demonstrate lifecycle repair/requalification rather than only one successful disassembly.

## 9. Decision rule

The robot-assisted route is not “better” because it is faster on one operation. A defensible improvement is a measured Pareto improvement in at least one declared outcome (accepted repair output, total crew burden, damage, repair cycle time, service interruption or verified component recovery) **without degrading** safety, required function, durability, requalification validity or worker outcomes.

No universal numeric target is assigned before the R0 baseline and variance are measured. No single composite score may conceal hard-gate failure or shift burden to the human operator.

## 10. Claim ceiling

This protocol applies only to the declared panel article, connection families, tools, fixtures, workcell, procedure and test method. It does not qualify a structural system, a building, an occupied-site robot, a reusable component across other applications, or regulatory approval.
