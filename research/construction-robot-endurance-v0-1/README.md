# Construction Robot Endurance and Maintainability Protocol V0.1

**Issue:** [#3628 — sustained accepted production, wear and maintainability benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/3628)  
**Profile:** `profile/construction-robot-endurance-v0.1.json`  
**Schema:** `schema/construction-robot-endurance-study-v0.1.schema.json`  
**Status:** proposed research protocol; no endurance campaign has been performed by Symtropy.

## Purpose

A construction robot must be evaluated over sustained operation, not only its fastest isolated cycle. This profile defines the study boundary for cumulative cycles, wear, planned maintenance, unplanned failure, recovery, material replenishment and accepted output across multiple shifts.

It is a study-plan and evidence-binding contract. It is not a fleet scheduler, CMMS, robot controller, safety case, certification or operational authorization.

## Industry evidence

### Built Robotics Gen 1 — Goorambat East pilot (2025)

ARENA's public lesson-learnt report records 1,723 piles installed during the 5 MW Goorambat East pilot between 24 February and 1 April 2025. It also describes hardware wear/fatigue and increasing downtime at higher cycle volumes, together with site logistics, pile delivery, handling-equipment and support dependencies. This is a pilot lesson, not an independently controlled lifetime reliability study.

Source: https://arena.gov.au/assets/2025/06/Built-Robotics-Australian-Robotic-Piling-Deployment-for-Large-Scale-Solar%E2%80%AF-Lesson-Learnt-Report-1.pdf

### Fortescue / Built Robotics / ARENA Cloudbreak trial (Aug–Oct 2025; report published 27 February 2026)

The report records basket capacities of roughly 120–224 piles and replacement times of around 20–30 minutes; CSF timing/hardness problems and cracking created rework; direct-drive and >40°C summer conditions were not demonstrated. These facts expose replenishment and workflow effects that a single-pile cycle benchmark misses. They are not a machine lifetime or universal MTBF result.

Source: https://arena.gov.au/assets/2026/03/Fortescue-Pilbara-Solar-Innovation-Hub-Built-Robotics-Trial-Report.pdf

### Broader field-deployment evidence

A 2026 systematic review of 75 real-world construction-robot deployments identified environment (45 cases), system integration (44), hardware (39), cost/scalability (15) and human–robot interaction (14) as recurring failure dimensions. These are review-coded case counts, not prevalence estimates for all robots. They justify testing the robot and its site/workflow dependencies together.

Source: https://www.sciencedirect.com/science/article/pii/S2666165925002418

## Paradigm shift: sustained accepted production

The primary outcome is the time-integrated rate of independently accepted task-specific functional units under a declared duty cycle. Report raw state durations and failure/maintenance events alongside final accepted output. Do not collapse uptime, availability, reliability, maintainability, quality and productivity into one score.

## Campaign identity and exposure

Every campaign binds:
- exact robot manufacturer/model/serial, firmware/software, configuration, payload/tool and service revision;
- task profile and accepted-output ledger/profile;
- site, work package, ODD stratum, environment, crew/operator configuration and site epoch;
- planned duty cycle, shift/calendar schedule, service windows, cumulative cycle/hour target and stopping rules;
- timebase/clock identity, event ordering and sensor/telemetry health;
- event definitions, failure adjudication rules, censoring policy and missing-log policy.

Record exposure separately by powered hour, task-eligible hour, robot operating hour, cumulative cycle and accepted functional unit where applicable. Do not use any one exposure denominator for every failure mode. An event rate without a declared exposure denominator is not estimable.

## Time-state model

The study profile defines mutually exclusive robot states for interval accounting:
- not scheduled;
- scheduled but no eligible work package;
- ready/available;
- working/executing;
- idle/awaiting operator or upstream task;
- charging/refuelling/changeover;
- blocked by site/material/other external cause;
- planned maintenance;
- unplanned maintenance;
- fault;
- recovery;
- safety hold;
- unknown/unobserved.

Every scheduled interval must reconcile to exactly one state or an explicit unknown interval. Store source timestamps and declared monotonic clock domain. State durations are operational facts; their classification does not adjudicate whether a safety hold or maintenance action was unnecessary.

Report gross scheduled-time state distribution and task-eligible state distribution separately. Exclusions such as no work package, weather stop, site access or permit delay must remain visible.

## Failure, wear and maintenance event model

Record separately:
- planned preventive maintenance and inspection;
- unplanned failure/fault and derating;
- diagnostic intervention and reset;
- wear-part replacement and adjustment;
- consumable/material replenishment;
- calibration or alignment drift;
- recovery/return-to-work verification;
- repeat or recurring failure mode;
- externally blocked operation and site integration failure.

Each event binds start/end times, cumulative exposure at onset, affected subsystem/component, mode/severity, detection and attribution status, evidence, service/repair action, parts/consumables, responsible role and post-service verification. Distinguish confirmed failure, suspected degradation, planned service, external blocker and unknown cause.

Do not infer root cause from a fault code alone. Preserve contested/unknown attribution and evidence references.

## Metrics — independent dimensions

1. **Accepted production:** finally accepted units per robot-hour, direct crew-hour and total workflow crew-hour; use #3605 and the task-specific functional-unit profile.
2. **State occupancy:** scheduled/eligible duration by every time state above, with unknown and exclusions reported.
3. **Operational availability:** publish an explicitly named numerator/denominator and exclusions; report both gross scheduled-time and task-eligible availability where meaningful. Do not import one universal availability convention.
4. **Reliability:** mode-specific failure counts/rates per declared cycles or hours, recurrence, derating and exposure at risk. Use time-to-event/recurrent-event methods appropriate to censoring and clustering.
5. **Maintainability:** time to detect, diagnose, repair and restore; report distributions, censoring and service classes.
6. **Wear/degradation:** change in cycle time, energy, geometry/installation quality, intervention frequency and maintenance burden versus cumulative exposure, load and environmental strata.
7. **Lifecycle burden:** labour, downtime, service, parts, consumables, transport, energy/fuel and rework per finally accepted unit; full costs belong to #3631.

A few zero-failure test hours do not establish an MTBF. Use exact exposure and confidence bounds only when event assumptions are justified. Pile installs, sensor frames, controller ticks and repeated logs are not interchangeable independent trials.

## Experiment design

- Separate familiarisation and break-in from scored operation.
- Preregister duty cycle, workload distribution, environment, shift lengths, planned maintenance, failure modes, exclusions and censoring.
- Use multiple serial units and configurations where feasible. One serial unit does not establish fleet reliability.
- Include repeated shifts, load variations, routine replenishment and planned servicing; specify actual cumulative cycles/exposure rather than invent a universal campaign length.
- Retain every outage, fault, derating, intervention, incomplete task and missing telemetry interval.
- Keep observational endurance separate from safely staged fault-injection/abnormal-condition tests.
- Do not defeat manufacturer service intervals or protections to create a longer run.
- Use matched conventional workflow for accepted project output, not only machine-on time.
- Cluster by serial unit, crew, shift, site and environment; report censoring and missingness.

## Safety separation

This profile can record safety holds and interventions but cannot declare them avoidable, change their disposition or authorize restart. Any unsafe degradation of braking, sensing, stability, load handling, communication or recovery remains within the separate physical safety qualification and task-specific risk assessment. Return-to-service requires the responsible authority's own evidence-backed process.

## Initial protocol-quality targets

- 100% of scheduled time reconciles to a declared state or explicit unknown interval.
- 100% of failure/maintenance/intervention events bind exact serial, configuration, site, task and event time.
- 100% of accepted production derives from independently adjudicated output units, not machine counters.
- No failure-rate/MTBF claim without mode definition, exposure denominator, censoring, clustering and uncertainty.
- No fleet/lifetime transfer claim from one serial unit or one site.
- Every post-maintenance/recovery return-to-service references the owning safety/operational authorization process; this benchmark does not provide it.

## Ownership

- #3628 owns endurance study design and reliability/maintainability reporting.
- #3605 owns accepted-output and workflow labour accounting.
- #3591 owns round-robin metrology/reproducibility.
- #3523/#3524, #3626/#3627, #3629/#3630 own task-specific output adapters.
- #3505/#3506, #3317/#3499/#3542/#3558/#3566 own separate safety, ODD, observation, mission and chain qualification.
- #3521/#3522 own physical trial receipts/raw evidence references.
- #3631 owns lifecycle cost per accepted output.
- #3531 and Symthaea #4918 own claim-specific evidence admissibility and dependency lineage.

This protocol does not create a CMMS, fleet scheduler, safety controller, event store or generic evidence authority.
