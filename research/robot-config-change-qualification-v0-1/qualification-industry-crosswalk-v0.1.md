# Robotics qualification — industry and standards crosswalk v0.1

Status: research crosswalk for ROBOT-CONFIG-CHANGE-QUAL-001. Checked 16 November 2026. This is not a conformity assessment, legal opinion, certification plan or endorsement of a product.

## Executive position

The field already has strong foundations in robot performance assessment, mobile-robot metrology, test-configuration recording, fleet interoperability, industrial robot safety, and task-specific commercial systems. Luminous should build on them.

The differentiated research target is narrower:

> **Given a precisely identified robot configuration change, determine which claim-specific evidence remains applicable, what must be reviewed or reverified, and how to prove that the minimum competent requalification was performed.**

That is a lifecycle evidence/applicability layer across existing robotics performance and safety methods—not a replacement for those methods.

## 1. Standards, test methods and interoperability

### ASTM F3327-23 — record the A-UGV test configuration

ASTM F3327-23 already provides a practice for recording A-UGV test configuration so results can be contextualized and tests recreated. Its public scope and significance notes cover, among other things, hardware make/model/part/serial/revision, drive/steering type, loaded state, software/firmware versions, velocity and acceleration limits, safety-sensor thresholds, maps, localization methods, sensors and test equipment.

**What we should do:** ingest or map the applicable F3327-23 test-configuration fields into our exact configuration manifest. Keep source identity, revision and raw vendor/test record alongside our normalized projection.

**What it does not replace:** a claim-specific dependency assessment deciding whether evidence from a prior configuration can be carried forward after a change. The standard itself warns that A-UGVs can differ materially in configuration and capability, making contextualized recording essential.

Source: [ASTM F3327-23 — Standard Practice for Recording the A-UGV Test Configuration](https://store.astm.org/f3327-23.html).

### AMRA-201:2026 — mobile robot requirements and test methods

AMRA-201:2026 was published on 26 July 2026. It establishes general requirements and test methods for mobile robots operating on solid travel surfaces and covers wheeled, tracked, legged and other qualifying mobile robots, including charging stations.

**What we should do:** map its applicable test methods and reporting vocabulary into platform-specific benchmark profiles. Treat its scope and conditions as explicit, not as a universal construction-robot approval.

**What we should not claim:** that AMRA-201 supplies a general configuration-change impact oracle, or that matching one test method proves suitability for every construction environment.

Source: [AMRA-201:2026 — Mobile Robots: General Requirements and Test Methods](https://amr-alliance.org/amra-201/).

### NIST — compositional performance and mobility metrology

NIST's Performance Assessment Framework for Robotic Systems explicitly composes perception, mobility, dexterity and safety measurements into task/system performance models. Its Mobility Performance of Robotic Systems programme develops measurement science, common reference measurements, metrics, testbeds, test methods and datasets for mobile and wearable robotics, including stability, adaptability, task efficiency, task quality and assurance.

**What we should do:** use NIST's compositional framing for the platform-level mission and its measurement-science discipline for independent reference measurements. Our added research layer is to bind these measurements to claim-specific configuration lineage, common-mode dependencies, evidence currentness and requalification cost.

**What we should not claim:** that we invented compositional capability assessment, task-level performance models or mobile-robot metrology.

Sources:
- [NIST — Performance Assessment Framework for Robotic Systems](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems)
- [NIST — Mobility Performance of Robotic Systems](https://www.nist.gov/programs-projects/mobility-performance-robotic-systems)

### VDA 5050 v3.0 — fleet communication, not qualification

VDA 5050 v3.0 was released in April 2026. It extends the mobile-robot communication interface for mixed fleets, including free-navigation zones, path sharing and power-saving actions.

**What we should do:** add a differential interoperability lane when the reference platform reaches fleet integration. Verify message compatibility, zone interpretation, path sharing, state/error reporting and version negotiation against the published interface.

**What we should not infer:** VDA 5050 conformance does not establish physical capability, stopping performance, task quality, functional safety, evidence independence or safe operation in a particular site.

Sources:
- [VDA — Version 3.0 of VDA 5050 released](https://www.vda.de/en/press/press-releases/2026/260421_PM_VDA_5050_EN)
- [VDA — current VDA 5050 version](https://www.vda.de/en/topics/automotive-industry/vda-5050.)

### ISO safety-scope crosswalk

- **ISO 10218-2:2025** addresses integration of industrial robot applications and robot cells across design, integration, commissioning, operation, maintenance, decommissioning and disposal. Its published scope explicitly excludes hazards relating to mobility when robots/manipulators are integrated with driverless industrial trucks or mobile platforms.
- **ISO 3691-4:2023** specifies safety requirements and verification for driverless industrial trucks and their systems; its examples include AGVs and AMRs. Its stated exclusions and operating-environment assumptions matter. It is not a universal construction machinery standard.
- As of 16 November 2026, **ISO/DIS 3691-4, Edition 3** is under development at the DIS/FDIS transition stage. Do not treat the draft as the published current standard until the edition is formally published.

**What we should do:** create a project-level applicability matrix for the actual robot class, integration arrangement, operating environment, intended use, hazards and jurisdiction, then get the applicable requirements assessed by competent safety professionals. Map our configuration/evidence records to the selected standards; do not substitute our benchmark for the safety assessment.

Sources:
- [ISO 10218-2:2025 — Industrial robot applications and robot cells](https://www.iso.org/standard/73934.html)
- [ISO 3691-4:2023 — Driverless industrial trucks and their systems](https://www.iso.org/standard/83545.html)
- [ISO/DIS 3691-4 — Edition 3 under development](https://www.iso.org/standard/88615.html)

## 2. Commercial comparison — task-specific, not leaderboard theatre

Published specifications are useful for setting test conditions and identifying what a buyer values. They are not matched, independent head-to-head performance trials. Keep vendor-stated conditions and claims separate from common-referee measurements.

| System | Publicly stated reference point | Correct comparison boundary |
| --- | --- | --- |
| Dusty Robotics FieldPrinter 2 | Manufacturer specifies ±1/16 in (±1.6 mm) print accuracy given accurate surveyed control; the system is operated by one operator. | Compare layout accuracy distribution, coverage, accepted output, site setup, control-network dependence, changeover, consumables, uptime and total crew-hours under matched site/CAD conditions. |
| HP SitePrint | Manufacturer specifies ±3/32 in (±2 mm) layout tolerance and ±1/32 in (±0.8 mm) average floor-level accuracy under a specified high-accuracy setup, 5–30 m range, SMR prism and 1-inch total station. Its headline productivity claim must be separated from its footnotes; the cited 30% figure is an internal comparison with its previous software version, and actual gains vary by application. | Reproduce the specified setup first, then measure independently with a separate common-referee track. Report layout and floor-level results separately. |
| Built Robotics RPD35 / Fortescue Cloudbreak trial | ARENA describes a site trial of autonomous piling at Cloudbreak, intended to assess pile placement, Pilbara conditions, pre-drilling/CSF backfill integration, manning and operating efficiency. | This is a different work package from floor layout or survey. Compare only against matched piling/earthworks baselines, including ground preparation, pile type, rejection/rework, logistics, crew/equipment support and environmental strata. |

Sources:
- [Dusty Robotics — FieldPrinter specifications](https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs)
- [HP — SitePrint product information and specification footnotes](https://www.hp.com/us-en/printers/site-print/layout-robot.html)
- [ARENA — Fortescue Solar Innovation Hub Built Robotics Trial Report](https://arena.gov.au/knowledge-bank/fortescue-solar-innovation-hub-built-robotics-trial-report/)

### The right competitive question

Do not ask only whether our robot is more accurate or more autonomous. Ask:

- How much finally accepted work does the complete system produce per total crew-hour and per elapsed site-hour?
- What is the distribution of error and the share of output inside the declared acceptance tolerance?
- How many outputs are rejected, repaired, reworked, unmeasured or unresolved?
- How much setup, survey control, calibration, repositioning, charging, replenishment, supervision and recovery is required?
- How long does a module change take from last accepted output A to first independently accepted output B?
- Which previous claims remain applicable after that change, and what is the competent minimum set of checks?
- Can a local technician diagnose, repair, calibrate and verify the module with locally available tools and spares?
- Can another laboratory reproduce the measurement and the evidence disposition on a held-out configuration?

For all three vendor/product examples, keep vendor-published specifications distinct from independently measured results and from our synthetic research fixtures. No synthetic result can establish superiority over any commercial system.

## 3. Common-referee experiment for the Luminous reference workcell

The initial demonstrator should remain a low-consequence, controlled and reversible workcell: one mobile base, a survey/inspection module, and a non-load-bearing layout/marking module. It should not begin with excavation, lifting, cutting, fastening, or autonomous work around the public.

### Matched task protocol

Freeze, hash and publish the test manifest before scoring:

- identical CAD/BIM/task input and acceptance geometry;
- identical surveyed site control, reference frame and site epoch;
- declared surface, lighting, obstruction, temperature and environmental conditions;
- identical start/finish boundaries and permitted operator interventions;
- declared training/familiarization policy and operator experience strata;
- exact robot serial, as-built configuration, tool, payload, firmware, calibration and battery state;
- same work package, acceptance criteria, independent instruments and uncertainty budget;
- preregistered treatment of aborted, incomplete, rejected, reworked and safety-stopped attempts.

Run two measurement tracks:
1. **Vendor-native track:** reproduce each system's documented preferred/required setup and method.
2. **Common-referee track:** independently measure all systems against the same reference network, calibrated instruments and blinded output adjudication where practical.

The common referee should not rely solely on the robot's own localization, its own print/placement estimate or a dashboard export generated by the system being assessed. Capture raw observations and preserve shared reference/calibration/clock dependencies under the #4918 independence model.

### Report these endpoints separately

1. **Geometry and quality:** signed bias, absolute-error distribution, P50/P95/P99, maximum observed error, uncertainty, coverage, missed/duplicated features, first-pass and final acceptance.
2. **Productivity:** accepted functional units per robot-hour, direct crew-hour, total workflow crew-hour and elapsed site-hour; include set-up, survey, support equipment, staging and waiting.
3. **Reliability and recovery:** task completion, safety stops, faults, interventions, diagnosis/repair/restore time, repeat faults and unresolved intervals.
4. **Resource use:** energy per accepted unit, consumables, transport, support equipment, spare-part consumption and operator burden.
5. **Reconfiguration:** changeover time, number and duration of tests, false invalidations, false preservations, review effort, and time to first accepted target output.
6. **Evidence quality:** configuration completeness, calibration/measurement validity, common-mode independence, evidence age, cross-lab reproducibility, provenance and claim-level applicability.
7. **Lifecycle:** degradation and accepted output by accumulated cycle/duty stratum, maintenance burden and local serviceability.

Do not use uptime as a proxy for productivity. Do not average away rejected work, safety stops, uncertainty or site blockers.

## 4. How to beat the field responsibly

The proposed competitive sequence is:

1. **Reach parity on a narrow task.** Reproduce commercial setup conditions and compare independently before claiming a technical advantage.
2. **Win on lifecycle economics.** Demonstrate more finally accepted output per total workflow hour, including support equipment, rework, calibration and maintenance.
3. **Win on module interchangeability.** Show that a module can be replaced without losing unrelated evidence, while all newly affected safety/quality claims are correctly held for review or re-verification.
4. **Win on repairability.** Show local replacement, service tools, documentation, spares and verified return-to-service rather than maximizing sealed proprietary autonomy.
5. **Win on generalization.** Publish a seeded configuration distribution, test unseen combinations, replicate across articles and laboratories, and expose benchmark shortcuts.
6. **Win on trustworthy evidence.** Make every capability assertion trace to an exact article/configuration, admissible evidence and operating envelope; preserve unresolved contradictions rather than averaging them away.

### Proposed qualification targets — research goals, not results

| Dimension | Prospective target |
| --- | --- |
| Critical claim preservation | Zero observed safety- or quality-critical false-preserves on the frozen adversarial corpus; every case and denominator published |
| Finite oracle correctness | 100% exact agreement on supported small synthetic dependency graphs |
| Affected-claim recall | ≥95% overall and 100% for predeclared safety/quality-critical claims on held-out cases |
| Requalification efficiency | ≥30% fewer unnecessary test actions than invalidate-all, with every hard gate retained |
| Expert effort | ≥20% lower median evidence reconstruction/review time than a documented manual baseline |
| Reproducibility | Same pinned inputs and evaluator produce byte-identical normalized outputs; independent implementation catches shared-oracle/parser errors |
| Stochastic generalization | Publish per-seed/per-configuration outcomes, interval estimates and power rationale; no rank claim when uncertainty cannot distinguish systems |
| Physical superiority | Only claim after task-matched, common-referee, independently replicated trials; report quality, accepted output and total workflow cost together |

A finite result of zero false-preserves is not a zero-risk claim. Report the number of independent configuration challenges and an uncertainty interval; repeated sensor frames, heartbeat ticks or multiple claims from one physical configuration are not independent trials. Physical safety conclusions require the separate hazard/risk and conformity processes.

## 5. Gaps we should own versus gaps we should compose

**Compose:** configuration recording (ASTM), mobile robot methods (AMRA), metrology/compositional assessment (NIST), communication (VDA 5050), physical safety standards (ISO/other applicable codes), trial receipts (#3522), admissibility (#3531), claim dependencies (#4918), capability matching (#1834), lifecycle (#3628/#4863), observation integrity (#3695) and drift/safe fallback (#4858/#4117).

**Own:** the robotics-specific adapter and benchmark that binds exact configuration deltas to claim-level evidence applicability; its independent oracle; the stochastic held-out challenge protocol; requalification cost and review-effort measurement; and a reproducible reference workcell experiment linking reconfiguration to accepted output.

This is deliberately an integration wedge rather than a new robot standard. Its usefulness comes from making existing standards, test methods and evidence owners work together across the full configuration lifecycle.

## References

- ASTM F3327-23: https://store.astm.org/f3327-23.html
- AMRA-201:2026: https://amr-alliance.org/amra-201/
- NIST Performance Assessment Framework: https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems
- NIST Mobility Performance: https://www.nist.gov/programs-projects/mobility-performance-robotic-systems
- VDA 5050 v3.0: https://www.vda.de/en/press/press-releases/2026/260421_PM_VDA_5050_EN
- ISO 10218-2:2025: https://www.iso.org/standard/73934.html
- ISO 3691-4:2023: https://www.iso.org/standard/83545.html
- ISO/DIS 3691-4 Edition 3: https://www.iso.org/standard/88615.html
- Dusty Robotics FieldPrinter 2 specifications: https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs
- HP SitePrint: https://www.hp.com/us-en/printers/site-print/layout-robot.html
- ARENA Cloudbreak trial report page: https://arena.gov.au/knowledge-bank/fortescue-solar-innovation-hub-built-robotics-trial-report/
