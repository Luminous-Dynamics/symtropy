# ROBOT-CONFIG-MISSION-IMPACT-001 — commercial benchmark crosswalk v0.1

Status: research comparison, not a product test or a claim of superiority. Product specifications below are manufacturer-declared or agency-reported under their stated conditions. They are not independently reproduced by Luminous Dynamics.

Checked: 16 November 2026.

## Why these comparators

The first reference platform is a modular mobile workcell for survey/inspection plus a non-load-bearing layout/measurement module. The nearest commercial comparison is not “robot vs robot” at a generic motion-speed level. It is the entire survey-to-accepted-layout workflow, including source data preparation, site control, localization, obstacles, print/mark, floor measurement, inspection, correction, support and service.

A second adjacent comparison is autonomous piling: it tests whether we can generalize the same evidence and lifecycle method to a different construction process without pretending that millimetres of layout, installed piles and square metres of masonry share one output unit.

## Commercial reference points

### Dusty Robotics FieldPrinter 2

Dusty's 21 September 2026 specification describes FieldPrinter 2 as a full-scale CAD/VDC layout platform, with 1/16 inch (about 1.6 mm) print accuracy **given accurate surveyed control**, single-operator operation, a Leica Absolute Laser Tracker and integrated Dusty Portal/AutoCAD workflow. It specifies at least three visible non-collinear control points per station, recommends 50–150 ft control-point spacing, lists 0.75 m/s continuous-line print speed, 0.50 m/s text/point speed, and two 90 Wh batteries with 2–3 hours of runtime per battery. The system also supports obstacle scanning and floor elevation mapping.

Sources:
- Manufacturer spec, updated 21 September 2026: https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs
- Manufacturer control-point requirements, 13 October 2026: https://support.dustyrobotics.com/hc/en-us/articles/53221144967963-FieldPrinter-Operation-Requirements

Comparison discipline:
- Reproduce the surveyed-control and tracker visibility conditions, including point count, geometry, spacing, instrument setup and site range.
- Separately stress control-point degradation, blocked line of sight, frame/revision mismatch, obstacle density, battery change and remount/re-stationing.
- Do not compare line-print speed with final accepted layout productivity. Count features only after independent completeness, position, identity/frame and acceptance checks.
- Treat integrated VDC preparation, file transfer, Portal reporting, single-operator staffing and field control setup as part of the workflow, not as invisible overhead.

### HP SitePrint

HP's current official product page states a high-accuracy layout tolerance of ±3/32 inch (±2 mm) and average floor-level tolerance of ±1/32 inch (±0.8 mm) for the specified 5–30 m operating distance using the HP SitePrint SMR prism, High Accuracy Print Mode and a 1-inch robotic total station (the footnote lists tested Leica TS16/60, Topcon GT1201 and Trimble S9 instruments). HP also describes cloud-based management, support/repair coverage and next-business-day replacement. Its “up to 10x faster” statement is a product-level marketing claim and HP itself says not every project achieves it; its published software productivity improvements compare against its previous software version, not against an independently matched competitor in every site condition.

Sources:
- HP official product page and conditions/footnotes: https://www.hp.com/us-en/printers/site-print/layout-robot.html

Comparison discipline:
- Reproduce HP's total-station class, prism, print mode and 5–30 m working range for its declared high-accuracy condition.
- Keep layout tolerance and average floor-level measurement as different measurands and separate endpoints.
- Report total-station setup/visibility, cloud/CAD revision management, field crew, unit swap/support and repair availability alongside performance.
- Do not quote “10x” as a baseline unless the exact project/application, comparator workflow, work scope and labour denominator are independently matched.

### Built Robotics RPD35/RPS25 autonomous piling

The ARENA/Fortescue Cloudbreak trial report documents a bounded autonomous piling trial under pre-drilled, cement-stabilised-fill conditions. ARENA's public summary describes the trial as a real-world evaluation of operational performance, productivity, quality, safety and reliability in the Pilbara; the trial showed the technology could operate in those conditions, while the detailed report records important conditions and limitations. The report's reported staffing/logistics benefits are trial-specific and must not be generalized to untested ground, weather or installation methods.

Sources:
- ARENA knowledge-bank summary: https://arena.gov.au/knowledge-bank/fortescue-solar-innovation-hub-built-robotics-trial-report/
- Full trial report: https://arena.gov.au/assets/2026/03/Fortescue-Pilbara-Solar-Innovation-Hub-Built-Robotics-Trial-Report.pdf

Comparison discipline:
- Stratify by ground class, pre-drilling, fill treatment, pile type, embedment, survey/control, weather, shift length, crew policy and QA method.
- Count installed piles only after the customer's complete acceptance predicate and traceable installation/inspection records.
- Include material handling, basket replenishment, equipment relocation, remote monitoring, operator coverage, repairs, remobilization and ground preparation.
- Keep “autonomous drive/installation speed” separate from accepted piles per total workflow crew-hour and cost per accepted pile.
- Do not treat a trial result under prepared ground as proof for untreated rock, wet ground, night shift or other installation processes.

## Test tracks: fair comparison without erasing vendor envelopes

### Track A — declared-condition reproduction

For each vendor, reproduce the exact published configuration and conditions to the extent possible. Bind instrument model/accuracy, control network, site range, surfaces, configuration/software versions, crew, work mix, environment, acceptance rule and exclusions. Independently measure the output and disclose any conditions that could not be reproduced.

This answers: “Can we independently reproduce the performance the vendor declares under the vendor's own stated conditions?”

### Track B — common-referee comparison

Run the same preregistered work distribution, design revision, site/control network, task acceptance profile and independent reference process across candidate systems where their operating envelopes overlap. Preserve each product's allowed setup and native workflow. Stratify or exclude only on predeclared applicability grounds; do not silently force one system into another's operating envelope.

This answers: “Under shared conditions that are genuinely applicable to both, which complete workflow delivers more accepted work?”

### Track C — boundary and degradation

Vary the operating conditions within a predeclared safe test envelope: reduced control visibility, surface variation, obstacles, frame changes, sensor remount, battery depletion/replacement, tool changes, operator turnover and calibration expiry. Out-of-envelope/unknown cases test correct abstention and safe hold, not a contest to see who violates constraints most aggressively.

This answers: “What changes first, how is it detected, and what evidence must be renewed?”

### Track D — reconfiguration and lifecycle

Measure the interval from last independently accepted output in A to first accepted output in B (owner #5411), and relate mission consequences to claim-specific evidence applicability (#5488), accepted output (#3605), maintenance/restoration (#3628), and terminal reserve/future-option retention (#11866). Report changeover, requalification, recommissioning, first-article hold/rejection, rework, repair, service delay and restoration separately.

This answers: “Can the platform be changed and returned to accepted work with less avoidable downtime and less repeated testing—without retaining stale evidence?”

## Required common report

Do not reduce this to a single weighted leaderboard. Report at least:

1. **Accepted work:** raw planned/attempted/produced/inspected/accepted/rejected/held/reworked/unknown units by task family and work class.
2. **Quality:** independent reference, calibration, measurand, signed error, uncertainty, acceptance tolerance, completeness and first-pass/final disposition.
3. **Productivity:** accepted units per robot-hour, direct crew-hour, total workflow crew-hour and elapsed site-hour, with every numerator and denominator.
4. **Workflow burden:** survey/control setup, source-model/VDC preparation, file transfer, calibration, station moves, obstacle handling, charging, consumables, inspection, rework and closeout.
5. **Robustness:** outcome distribution across common seeds, independent seeds, sites, articles, operators, reconstruction teams and laboratories.
6. **Lifecycle:** failures, wear, maintenance, MTTR, replacement supply, service support, requalification and terminal reserve vector.
7. **Change cost:** exact claims invalidated/preserved/reviewed, evidence requests, requalification actions, time to first accepted output and avoidable retest.
8. **Safety:** separate hard-gate results, stop/hold events and applicable authority. Never trade safety outcomes against productivity.

## What would count as doing better

A defensible “better than SOTA” result requires all of the following:
- a matched, preregistered task and work distribution with correct functional units;
- exact vendor configuration and operating conditions;
- independent measurement with an uncertainty budget capable of resolving the acceptance margin;
- independent acceptance and full workflow labour/resource accounting;
- a sufficient number of independent seeds/articles/labs, with paired and unpaired strata analysed correctly;
- held-out distribution and benchmark-gaming probes;
- repeated results outside the development team;
- non-inferior or superior safety/quality hard gates;
- all changes, exclusions and failures disclosed.

The first ambition should not be to beat Dusty's 1.6 mm, HP's condition-specific ±2 mm layout claim or Built's prepared-ground piling trial on a synthetic fixture. It should be to publish a reproducible test-and-evidence system that can compare those products fairly, expose where their claims apply, quantify complete workflow economics, and measure how safely and cheaply each platform adapts after configuration changes.

## Standards and measurement-science alignment

- NIST's Performance Assessment Framework already composes perception, mobility, dexterity and safety component measurements into system-level task performance: https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems
- NIST's Mobility Performance programme develops measurement science and test methods for dynamic mobile systems, including slippage, uneven floors, obstacles, vibration, dynamic loads and delocalization: https://www.nist.gov/programs-projects/mobility-performance-robotic-systems
- AMRA-201:2026 provides general requirements and test methods for mobile robots on solid travel surfaces, including wheeled, tracked and legged embodiments: https://amr-alliance.org/amra-201/
- VDA 5050 v3.0 supports cross-vendor fleet communication and zone/path-sharing for more autonomous mobile robots; it does not qualify physical capability or safety: https://www.vda.de/en/press/press-releases/2026/260421_PM_VDA_5050_EN

Use these methods and interfaces rather than claim to replace them. This benchmark's distinct contribution is the evidence-preserving join across configuration change, complete-mission diagnostics, independent accepted output, lifecycle and requalification.

## Nonclaims

No Luminous robot has been independently benchmarked against Dusty, HP or Built. No vendor product test was performed in this tranche. The comparisons above are public-source research inputs only. No commercial superiority, standards conformity, safety certification or procurement recommendation is established.
