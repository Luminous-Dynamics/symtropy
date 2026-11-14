# Autonomous Solar Piling — Accepted Output Adapter V0.1

**Issue:** [#3626 — autonomous solar piling accepted-output benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/3626)  
**Generic contract:** [#3605 — accepted output and total-workflow productivity](https://github.com/Luminous-Dynamics/symtropy/issues/3605)  
**Profile:** `profile/solar-pile-accepted-output-v0.1.json`  
**Status:** task-adapter proposal; no pile installation has been benchmarked by Symtropy.

## Purpose

Define one task-specific functional unit for autonomous solar piling:

> One finally accepted installed pile, of a declared pile class and work-package revision, which meets the engineer-of-record's frozen project and geotechnical acceptance predicate and is supported by independent as-built/QA evidence.

A controller's `pile_complete` event, a drive cycle, a commanded pile count or an installed-pile counter is not an accepted output.

This adapter consumes the generic accepted-output accounting model. It must not create a second project-productivity ontology, safety controller, geotechnical solver, survey engine or evidence ledger.

## Industry evidence — distinguish separate deployments

### Built Robotics Gen 1 — Goorambat East 5 MW pilot (2025)

ARENA's public lesson-learnt report records deployment from 24 February to 1 April 2025 and 1,723 piles successfully installed. This is a milestone/pilot result, not a matched independent comparison across machines, soils, sites or project workflows.

Source: https://arena.gov.au/assets/2025/06/Built-Robotics-Australian-Robotic-Piling-Deployment-for-Large-Scale-Solar%E2%80%AF-Lesson-Learnt-Report-1.pdf

### Fortescue / Built Robotics — Cloudbreak trial (2025; report 27 February 2026)

The Fortescue Pilbara Solar Innovation Hub report records an RPD35 autonomous pile driver and RPS25 pile stabiliser trial at Cloudbreak. The trial ran August–October 2025 and the report was published 27 February 2026.

Important scope and findings:
- The trial objectives included installation of a significant number of piles, up to 1,500. The published report must not be paraphrased as proving that 1,500 piles were completed.
- Hard ground prevented the planned direct-driving method; pre-drilling and cement-stabilised fill (CSF) were used for all pile holes.
- The report states an average cycle of 2.61 minutes per pile, including pile retrieval, positioning and driving.
- It reports approximately 67% lower manning per piling rig than conventional methods and says one operator could supervise up to three rigs.
- Pile baskets held roughly 120–224 piles depending on pile size; replacement took around 20–30 minutes.
- CSF timing and hardness created performance issues; some CSF cracking caused rework.
- Project QA reported position, verticality, embedment and mechanical performance within project tolerances when CSF hardness was controlled.
- Summer extremes above 40°C and direct-driving conditions were not demonstrated by the trial.
- A direct cost comparison was not practically determined because of test scale and contractual/methodological differences.

Source: https://arena.gov.au/assets/2026/03/Fortescue-Pilbara-Solar-Innovation-Hub-Built-Robotics-Trial-Report.pdf

These two deployments are separate evidence items with different sites, dates, workflows and scopes. Do not combine the 1,723-pile 2025 pilot result with the 2025 Cloudbreak trial cycle time or labour observations as though they were one campaign.

## Acceptance predicate — project-specific, frozen before work

The engineer-of-record / geotechnical authority must supply the named acceptance profile and limits for the exact pile design, soil stratum and installation method. This adapter defines required evidence categories, not universal engineering tolerances.

Each pile work unit should bind:

1. **Identity:** pile ID, design revision, pile class/type, dimensions, material/heat/batch identity where specified, work-package and foundation-point IDs.
2. **Location:** approved coordinate frame, control/survey epoch, target position and independent as-built XY position with uncertainty.
3. **Installation method:** direct drive, pre-drill, CSF-assisted or other approved method; record changes/deviations and the reason.
4. **Ground conditions:** declared geotechnical/soil stratum, refusal/hardness state, groundwater/moisture and relevant pre-drill/CSF state.
5. **Geometry:** as-built location, verticality/inclination, embedment/depth and any specified orientation or offset criteria.
6. **Installation evidence:** depth/displacement trace, hammer/drive energy or force data where required by the design/QA plan, interruption/refusal events and final machine disposition.
7. **Mechanical/structural acceptance:** project-required load tests, capacity/verification records, damage/cracking checks and the engineer's final disposition.
8. **Independent inspection:** survey/metrology, inspection authority/method, calibration/uncertainty and raw evidence references.
9. **Attempt history:** each drive attempt, adjustment, re-drive, extraction, replacement, abandoned pile, defect and reinspection is linked to the original pile ID.

Do not infer capacity from depth alone, nor infer acceptance from the robot's internal displacement or force trace. The project specification determines which tests are required and which instruments/reference methods are competent for them.

### Outcomes

Preserve separate states:
- planned / not attempted;
- attempted / incomplete;
- installed, awaiting independent inspection;
- first-pass accepted;
- rejected / defective;
- reworked or re-driven / resubmitted;
- finally accepted;
- finally rejected;
- cancelled, superseded or unresolved.

A re-driven or replaced pile does not create additional accepted output. A replacement must have its own physical identity while retaining lineage to the failed/original pile and the design decision that authorised replacement.

## Full-workflow resource accounting

Separate robot operating hours, direct crew-hours and total workflow crew-hours. Include, where applicable:

- design/site coordination, survey control and pile set-out;
- access preparation, site logistics, mobilisation and machine footprint constraints;
- pre-drilling, spoils management, CSF mixing/delivery/pouring/curing;
- pile delivery, staging, basket loading/replenishment, telehandler/forklift handling;
- robot/excavator setup, calibration, operator, spotter and remote supervision;
- robot cycle, repositioning, tool changes, maintenance, refuelling and waiting;
- inspection, as-built survey, verticality/depth QA and mechanical tests;
- CSF defects, extraction, re-drive, replacement and other rework;
- site changes, safety holds, exclusion-zone coordination, recovery and closeout.

Report:
- finally accepted piles per robot-hour;
- finally accepted piles per direct site crew-hour;
- finally accepted piles per total workflow crew-hour;
- elapsed time per finally accepted pile;
- first-pass yield and final acceptance with exact denominators;
- rework labour/equipment/material per accepted pile;
- position, verticality and embedment distributions by soil/pile/method;
- CSF cracking/damage, refusal, abandonment and replacement rates;
- machine availability, holds and recovery;
- fuel/energy/material consumption per accepted pile where measured.

The 2.61-minute reported cycle is not directly comparable to accepted piles per project crew-hour. It does not represent the full workflow boundary and must not be used as a cost or productivity verdict by itself.

## Matched baseline and experimental design

The preferred study is a randomized or balanced within-site crossover between autonomous and conventional methods on matched pile types, soil strata, work sequencing, preparation, CSF, shift conditions and acceptance criteria. Where randomization is infeasible, preregister the matching/counterfactual approach and expose confounding.

Stratify by:
- soil/geotechnical class, hardness/refusal and moisture/groundwater;
- direct-drive vs pre-drill/CSF method;
- pile size, material, length and pile class;
- temperature, dust, wind, slope and access;
- basket capacity/handling, telehandler support and site layout;
- machine serial unit, operator/remote-supervisor ratio, shift and crew.

The independent experimental unit is the pile installation attempt for pile-level acceptance, but pile attempts are clustered within machine, crew, shift and site. Repeated displacement samples, survey shots or controller ticks do not increase the independent number of pile installations. Site-transfer claims require a second site/soil/configuration and a cluster-aware analysis.

Do not attribute productivity gains to the robot alone when they arise from centralised supervision, pre-staging, CSF sequencing, revised site layout or changed work hours. Report each workflow intervention.

## Safety qualification remains separate

This adapter measures task quality and productivity only. Autonomous excavator-based piling has a different physical hazard envelope from indoor layout printing: heavy plant motion, pile handling, ground instability, exclusion zones, loss of communications, unexpected refusal and machine recovery all require task-specific risk assessment and physical safety qualification.

A small pilot with no reported incidents does not establish a safety failure probability. Reduced manning and improved crew productivity cannot compensate for a failed or unevaluated safety requirement.

## Initial benchmark contract targets

These are measurement/accounting requirements, not universal pile tolerances or achieved platform results:

- 100% of attempted piles reconcile to a finally accepted, rejected, reworked, replaced, abandoned, cancelled or unresolved state.
- 100% of finally accepted piles bind to the frozen design/geotechnical acceptance predicate and the required independent evidence.
- Zero productivity credit for uninspected, wrong-revision, wrong-frame or finally rejected piles.
- 100% of required identity, configuration, ground/method and time records reconcile to the work package.
- All first-pass and final acceptance rates publish raw numerators, denominators, strata, exclusions and independent-unit definitions.
- Matched productivity comparisons include survey, pre-drill, CSF, handling, supervision, QA, rework and recovery.
- No universal minimum productivity improvement is claimed. A ≥20% accepted-piles-per-total-workflow-crew-hour gain can be preregistered as an aspirational project hypothesis only after a baseline and pilot variance study.

## Ownership

- [#3626](https://github.com/Luminous-Dynamics/symtropy/issues/3626): task-specific solar-piling acceptance and matched benchmark.
- [#3605](https://github.com/Luminous-Dynamics/symtropy/issues/3605): generic accepted-output accounting.
- [#3591](https://github.com/Luminous-Dynamics/symtropy/issues/3591): round-robin metrology / reproducibility.
- [#1607](https://github.com/Luminous-Dynamics/symtropy/issues/1607): survey/control/frame/calibration/uncertainty.
- [#3521/#3522](https://github.com/Luminous-Dynamics/symtropy/issues/3521): physical trial receipts.
- [#3505/#3506](https://github.com/Luminous-Dynamics/symtropy/pull/3506): physical ODD and safety qualification protocol.
- [#3531](https://github.com/Luminous-Dynamics/symtropy/pull/3531): claim-specific evidence admissibility.

This profile is a research adapter. It does not implement a pile-driving controller or establish physical safety, structural capacity, regulatory conformity, certification, independent vendor superiority or actual field productivity.
