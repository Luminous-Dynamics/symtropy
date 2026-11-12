# Built Environment Competitive Baselines and Maturity Protocol v0.1

**Status:** research protocol; no comparative result is claimed.  
**Related:** Symtropy #4043 / #4062 and draft PR #4064; Sol-Atlas #502 / #549.

## 1. Purpose

Define how Luminous systems will be compared with current professional practice, commercial products and published research across building design, construction, inspection, reconstruction, maintenance and operations.

The benchmark must answer a stronger question than “does the platform have feature X?”:

> Under the same asset, task, operating envelope, information, workforce/resource budget and independent acceptance procedure, does the candidate produce better verified outcomes over the declared lifecycle?

Do not treat an unavailable competitor feature as absent. Use the dispositions COMPARABLE, PARTIALLY_COMPARABLE, NOT_COMPARABLE, UNASSESSED and SOURCE_NOT_AVAILABLE with reasons.

## 2. Industry evidence to anchor the comparison

### 2.1 Operational digital twins

A scoping review published 15 October 2026 examined 37 operational building digital-twin studies. It reports:
- indoor environmental quality monitoring in 73% of included studies;
- control-driving prescriptive maturity in 24%;
- quantitative outcomes reported in 27%.

These are descriptive findings within that review's eligibility and coding rules, not a universal census of deployed buildings. They motivate measuring what the twin actually does and what it improves, not merely whether it is called a digital twin.

Source: *Beyond the dashboard: Architecture, maturity, and performance outcomes in operational building digital twins*, Energy and Buildings (15 Oct 2026), https://doi.org/10.1016/j.enbuild.2026.117935

### 2.2 BIM and infrastructure lifecycle information

A systematic review published 30 October 2026 examined 97 BIM/infrastructure lifecycle studies. Interoperability and data modelling were the dominant information role in 65/97; digital twin/IoT/structural-health monitoring appeared in 49/97; open or semantic interoperability appeared in 47/97. The authors caution that technology labels do not demonstrate operational maturity and identify semantic continuity, updating/synchronization, governance and long-term validation as gaps.

Source: Pisciotta & Inzerillo, *Building Information Modeling for Infrastructure Lifecycle Management*, Applied Sciences (30 Oct 2026), https://doi.org/10.3390/app162110749

### 2.3 Robotic inspection and construction safety

A 2026 review of 135 UAV/deep-learning façade-inspection articles reports continuing problems with real-world robustness and with closing the semantic loop from detected defects to BIM-based maintenance decisions.

Source: Fan et al., *UAV and Deep Learning for Building Façade Defect Detection: A Comprehensive Review*, Sensors (22 Jun 2026), https://pmc.ncbi.nlm.nih.gov/articles/PMC13306407/

NIOSH's 7 October 2026 construction-robotics bulletin highlights dynamic work areas, multiple trades, dust/weather/lighting/occlusion, sensor degradation, communication/software faults, unexpected recovery behavior and human–robot interaction. These conditions belong in the operational design domain and test campaign, not only in a generic safety paragraph.

Source: NIOSH, *Safety in Construction Robotics* (7 Oct 2026), https://www.cdc.gov/niosh/bulletin/2026/safety-in-construction-robotics.html

### 2.4 Industrialized construction

Autodesk's 2026 survey reports that 84% of respondent organizations using industrialized construction say finished-project quality improved and 76% say schedule/cost predictability improved. These are respondent-reported outcomes, not randomized or matched causal estimates. Use them as industry context only, never as an external benchmark threshold.

Source: Autodesk, *Industrialized Construction Spotlight 2026*, https://www.autodesk.com/design-make/research/industrialized-construction-spotlight-report-2026

### 2.5 Robotic capability measurement

NIST's Performance Assessment Framework decomposes robot task performance into perception, mobility, dexterity and safety, and emphasizes composing component tests into task-driven system performance models. Reuse this foundation; extend it with construction-specific task context, accepted physical output, human/workflow burden and lifecycle maintenance.

Source: NIST, *Performance Assessment Framework for Robotic Systems*, https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems

## 3. Required comparator classes

Each benchmark family must select the relevant comparators from these classes:

1. **Best-current professional workflow:** qualified human specialist team using its normal tools, including manual inspection, engineering review, surveying, access equipment and documentation.
2. **Instrumented professional workflow:** the same work with the strongest appropriate measurement equipment and digital documentation, to isolate the value of automation from the value of better instruments.
3. **Commercial system:** a named product/version/configuration actually applicable to the task (BIM, digital twin, inspection robot, layout robot, repair robot, fleet manager or construction method).
4. **Best published task-specific research:** a reproducible method with sufficient source data and task definitions. A publication's reported metric is not automatically comparable.
5. **Luminous integrated pipeline:** exact versions of Symthaea, Symtropy, Sol-Atlas and Mycelix plus named external adapters, tools, robots and human roles.

No single vendor is the baseline for all tasks. Compare by function and claim surface. A full professional delivery team must not be compared with a small research adapter on feature count.

## 4. Match protocol

Before execution, freeze:
- asset/site identity and boundary;
- design/as-built revision, geometry, coordinate frame and units;
- task definition, functional unit and required output quality;
- defect prevalence and severity mix or construction work package;
- operating design domain, weather, lighting, access and site activity;
- sensors, robots, tooling, firmware, calibration and safety configuration;
- human roles, training, supervision, crew size and responsibility boundaries;
- information available to each system and any manual preparation effort;
- compute, energy, time, equipment, consumables and financial budget;
- independent reference instruments, acceptance tests and uncertainty budget;
- source, software, model, solver, dataset, evaluator and configuration digests;
- exclusions, censoring, missingness, stop conditions and failure handling.

Use randomized/blocked order where practical. Blind the referee to system identity where possible. Retain all failed, rejected, reworked, unresolved and uninspected units. Publish per-site/per-task results and uncertainty intervals. Do not pool heterogeneous task families into one headline score.

## 5. Operational digital-twin maturity ladder

This is a proposed Luminous evaluation taxonomy, not an existing industry standard.

| Level | Demonstrated capability | What it does not prove |
|---|---|---|
| T0 — Static representation | Model or drawing exists | Currentness or physical accuracy |
| T1 — Linked information | Asset identity and documents are connected | Correct semantics or valid evidence |
| T2 — Observed state | Sensor/inspection observations mapped to assets with coverage and provenance | Diagnosis or decision quality |
| T3 — Validated condition | Observations compared with qualified references; uncertainty and OOD are explicit | An intervention is beneficial or authorized |
| T4 — Decision support | Alternatives, hard constraints, resource feasibility and decision impact evaluated | Permission to execute |
| T5 — Authorized intervention thread | Human/organizational authority and execution prerequisites are explicit; exact work is tracked | Successful repair or return-to-service |
| T6 — Verified lifecycle loop | Independent post-work acceptance, recurrence monitoring, scoped requalification and lifecycle outcomes | Transfer to other assets/sites without separate evidence |

A project may be T4 for energy optimization but T1 for structural condition. Report capability by asset system and claim, not one global twin maturity label.

## 6. Result dimensions

### Design and engineering
- requirement completeness and change-impact detection;
- design alternative quality under hard constraints;
- independent solver agreement within declared context of use;
- model validation, uncertainty and domain applicability;
- material quantities, cost and lifecycle-carbon boundary.

### Construction and robotics
- task-level perception, mobility, dexterity and safety subtests;
- accepted functional output per robot-hour, direct worker-hour and total workflow crew-hour;
- first-pass yield, dimensional/functional conformity, rework and scrap;
- setup, travel, handling, access, material supply and site-interference burden;
- intervention, safe-stop, near-miss and recovery data;
- machine degradation, failure, maintenance and verified return-to-operation.

### Inspection and maintenance
- required-region coverage and unobserved fraction;
- defect detection, localization and severity classification per class;
- false-clear and false-alarm burden; calibration and selective abstention;
- time/cost to qualified decision and unnecessary interventions;
- independently accepted restoration, recurrence and service interruption;
- lifecycle maintenance burden and spares/skill constraints.

### City and human outcomes
- essential-service capacity and access;
- affordability, distribution of burdens and benefits, accessibility and displacement;
- energy/water reliability, resource consumption and emissions;
- climate/disaster resilience and recovery trajectories;
- worker exposure, ergonomics, cognitive workload, supervision and training;
- resident disruption and service continuity.

Keep safety, rights, code, accessibility and other designated requirements as hard gates. Do not allow cost, speed, carbon or a model score to compensate for a failed hard requirement.

## 7. Competitive success criteria

### P0 — contract correctness
- 100% agreement with an independent oracle on the finite frozen synthetic corpus;
- zero false acceptance of seeded hard violations;
- zero stale or wrong-context evidence promoted as current;
- zero silent semantic loss or unsupported-feature inference;
- deterministic replay from exact frozen inputs and tool versions.

### P1 — matched laboratory value
After reproducing the baseline and preregistering sample size, demonstrate a Pareto improvement in at least one primary outcome without degrading hard constraints or other declared protected outcomes. Report confidence intervals, per-case tails, failure classes and total workflow resources.

### P2 — multi-site transfer
Replicate across at least two independent sites/labs and multiple materials, geometry classes or robot configurations where the claim requires transfer. Report worst-stratum behavior and abstention.

### P3 — prospective operational value
Run a supervised prospective pilot with asset-owner consent, qualified professionals, worker participation and independently accepted outcomes. Track long-horizon maintenance, service performance, costs and actual material recovery.

Do not set fixed percentages for cost/productivity improvement before a trustworthy matched baseline exists. Stretch targets are useful for planning but cannot determine the evaluator's verdict.

## 8. What would constitute a genuine Luminous advantage?

A defensible advantage is not “one platform does everything.” It is a set of measured claims, such as:
- fewer engineering rework events because stale or conflicting inputs are detected before construction;
- fewer unnecessary inspections because measurements are chosen by decision impact;
- more complete inspection coverage with lower false-clear risk;
- lower total workflow burden for the same independently accepted output;
- shorter maintenance interruption without shifting work or risk onto workers;
- faster cross-vendor integration without semantic or authority loss;
- better material recovery and reuse with preserved provenance and verified condition;
- more robust access to essential city services through construction phases and disruptions.

Each claim must be tied to a named baseline, site/task, evidence tier, measurement method and independent evaluator.

## 9. Anti-patterns to reject

- Feature-count comparisons.
- Vendor marketing metrics treated as independently measured outcomes.
- Comparing different tasks, site conditions, quality requirements or crew boundaries.
- Counting robot motion, printed lines, predicted defects or completed work orders as accepted output.
- Treating BIM/IFC exchange as proof that semantics or physical truth were preserved.
- Treating a digital-twin label or live dashboard as closed-loop operational maturity.
- Reporting mean accuracy while hiding severe defect classes, worst regions or abstention.
- Converting missing data to zero, clear, safe or healthy.
- Treating a simulation, synthetic corpus or one physical article as evidence of city-scale or fleet-wide performance.
- Using one scalar score that permits safety, dignity, accessibility or durability to be traded away.

## 10. Next work

1. Add the matched-baseline manifest to the #502 Sol-Atlas industry comparison corpus.
2. Attach task-specific comparator profiles to the #4043 building lifecycle benchmark.
3. Reuse NIST perception/mobility/dexterity/safety methods for robot subtests.
4. Build the first synthetic known-answer corpus and independent oracle under #4062.
5. Only after P0 should the team commission a guarded non-load-bearing panel demonstrator.
