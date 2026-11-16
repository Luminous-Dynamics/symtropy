# ROBOT-REFERENCE-WORKCELL-001 — physical demonstrator design v0.1

Status: design/readiness only. No base has been selected, no hardware has been tested and no trial is authorized by this document.

Issue: https://github.com/Luminous-Dynamics/symtropy/issues/5662

## 1. Purpose

Build one modest, modular reference workcell to test whether configuration changes can be handled with less unnecessary requalification while stale or unsupported evidence remains difficult to accept.

The first product is not a new robot. It is a reproducible experiment linking:

last independently accepted output under A
→ safe stop and controlled module change
→ exact target configuration reconciliation
→ claim-specific requalification and competent verification
→ first independently accepted output under B.

This directly joins #5488 configuration-change qualification, #5411 reconfiguration chronology, #3605 accepted-output accounting and #5657 evidence-gated stage transitions.

## 2. Minimum configuration

### Mobile base

Select one commercially supportable wheeled base after a documented decision matrix. Required inputs:
- exact product/model/serial and configuration manifest;
- mass, centre of mass, dimensional envelope, payload limits and permitted mounting/load paths;
- wheel/drive and braking architecture, declared stopping data, surface/gradient limits and environmental ODD;
- safety controller, E-stop, protective devices, safety I/O and safe-state architecture;
- battery/power limits, thermal envelope, charging and reserve constraints;
- ROS 2 or documented integration interface, clock/timebase behavior and versioned drivers;
- service, spare-parts, repair and replacement availability.

The choice must be justified by the first experiment and supply/support realities, not by headline payload or speed alone.

### Module A — survey and inspection

A removable sensor assembly using suitable RGB-D, LiDAR and IMU components. The assembly has:
- separate serial identity for each sensor and compute/driver module;
- measured mount geometry and exact frame/transform identities;
- calibration profile, validity interval and raw calibration artifact;
- hardware/software/firmware revisions and clock-domain metadata;
- repeatable mechanical/electrical connector and installation procedure.

### Module B — non-load-bearing layout/measurement

A light, removable marker or measurement tool. It must not carry structural load, perform cutting/drilling, manipulate hazardous materials or operate near workers. Bind:
- tool serial, CAD/geometry digest, mount/TCP and connector identity;
- calibration/offset and installation repeatability;
- task-specific output unit and acceptance predicate;
- independent survey/referee method and uncertainty budget.

### Exclusions for v0.1

Do not alter or experiment with the braking controller, safety PLC, E-stop chain, protective-field logic, drive train or load-bearing structure. Do not perform excavation, piling, lifting, cutting, drilling, occupied-site autonomy or safety-critical production work. A later scope change requires new owner review, ODD and risk assessment.

## 3. Initial test matrix

| ID | Change | Hypothesis to test | Required evidence / hold condition |
| --- | --- | --- | --- |
| WCP-00 | No change / repeat baseline | Establish repeatability and measurement noise | Same frozen manifest, calibration and reference method |
| WCP-01 | Camera serial only; no calibration transfer | Sensor identity change must not inherit serial-specific calibration | New serial binding; hold claim until calibration and frame evidence admitted |
| WCP-02 | Sensor mount/extrinsic change | Only claims depending on mount/frame/localization/inspection should be reconsidered | Measured mount/extrinsic transform and independent frame check |
| WCP-03 | Lightweight tool geometry replacement | Reach/swept clearance and output-quality claims need distinct dispositions | Tool geometry/TCP, swept volume, process/acceptance evidence |
| WCP-04 | Compatible battery pack replacement | Energy/endurance requires health/capacity reassessment; unrelated geometry need not be repeated | Pack identity, health/usable capacity, mass/interface and power-bus compatibility |
| WCP-05 | Unknown or unclassified module dependency | Unknown dependency must not be preserved optimistically | Software-only negative control in v0.1; physical execution forbidden until owner review |

A “same model, new serial” is not proof of calibration equivalence. A nominally identical connector is not proof of interface compatibility. A compatible battery interface does not prove health or usable capacity. A synthetic equivalence witness is not physical evidence.

## 4. Measurement and comparator protocol

### Declared-condition reproduction

Where a commercial comparator has a published operating condition, reproduce the relevant instrument, control network, surface, site geometry, distance, software mode and workflow as closely as possible. Disclose deviations.

### Common-referee track

Compare systems only over the overlapping task/ODD envelope. Freeze task, site snapshot, design revision, reference frame, functional unit, acceptance tolerance and inspection method. Use an independent referee that does not reuse the robot's own unverified pose, map or calibration as ground truth.

For Module B, report:
- signed/absolute geometric error by feature class;
- reference method and uncertainty budget;
- completeness, identity/frame/revision correctness and critical omissions;
- attempted, produced, inspected, accepted, rejected, held, reworked and unknown units;
- first-pass and final acceptance.

Do not combine point, line, area, pile, drilled-hole or masonry units into one universal output metric.

### Reconfiguration and lifecycle track

Measure separately:
- last accepted A output;
- safe stop/isolation and WIP disposition;
- physical module swap;
- configuration/manifest reconciliation;
- calibration and commissioning;
- each required requalification action;
- first-article hold and inspection;
- first accepted B output;
- stable restart, repair/rework and return-to-service.

Report elapsed time, robot-hours and total workflow crew-hours separately. Include survey/control setup, source model preparation, calibration, software/driver update, consumables, charging, inspection, rework and closeout. Do not stop the clock at the first command acknowledgement or machine cycle.

## 5. Stage contract and safety boundaries

Each stage requires intent, exact configuration/task revision, preconditions, command/attempt identity, observed execution, externally testable postcondition, profile-appropriate verifier, raw evidence reference, uncertainty/currentness, disposition and downstream gate.

A controller's command-accepted, motion-complete or self-reported success event is not sufficient evidence of a postcondition. Unknown state remains unknown. Safety/quality-critical stage acceptance uses the verifier class required by the competent task profile; the robot does not authorize its own safety-critical release.

Before any physical actuation, require:
- competent risk assessment for the exact base, manipulator/tool and integrated system;
- restricted, flat, controlled test area with exclusion and safe-stop arrangements;
- verified E-stop/protective devices/safety I/O and controlled recovery procedure;
- declared safe state for module change and energy isolation;
- competent operator/maintainer roles and emergency response;
- reviewed site, surface, environmental, communications and human-presence ODD;
- independent referee and measurement uncertainty budget.

Standards applicability is a product-/use-/jurisdiction-specific decision. ISO 10218-1/-2 industrial robot/application requirements and ISO 3691-4 driverless-truck/AMR requirements have distinct scopes; none by itself proves this integrated mobile-manipulator workcell safe. See the commercial crosswalk for boundaries and current revision watch.

## 6. Statistics and reproducibility

The first commissioning/shakedown is a feasibility pilot, not a SOTA claim. Record all attempts and exclusions. Use common paired work packages for A/B comparison where reversible; counterbalance order when feasible. Independently vary seeds/sites and use an independent reconstruction team in subsequent replication.

Choose confirmatory sample size from pilot variance and a preregistered minimum detectable effect. Independent units should be article × task/site × configuration as appropriate; do not count telemetry frames, repeated stages or retry attempts as independent robot trials. Use clustered/hierarchical analysis where needed and report intervals, raw denominators and missing/aborted/unknown outcomes.

## 7. Required owner contracts before the trial

- #5488 — claim-specific evidence applicability and dependency-directed requalification;
- #5411 — transition receipt from last accepted A to first accepted B;
- #3605 — task-family functional unit, independent acceptance and labour/time denominators;
- #3523/#3524 — task profile, ODD and construction-performance baseline;
- #3522 — physical trial receipt, reference instruments, raw measurement and uncertainty;
- #3531 — claim-specific evidence admissibility;
- #3505 — physical safety/authority boundary;
- #3591 — inter-laboratory reconstruction, calibration and measurement comparability;
- #3628 — maintenance, wear, repair and verified restoration;
- #12046 — minimum competent evidence-request planning, if/when its stable adapter is published.

Missing owner interfaces remain explicitly unresolved; do not fabricate a receipt or use the synthetic fixtures as physical authority.

## 8. Release criteria for the pilot plan

Before a hardware trial is scheduled:
1. platform and module bill of materials is complete and reviewable;
2. interface and hazard/applicability map is accepted by competent owners;
3. test site, ODD and emergency/safe-state process are approved;
4. independent reference and uncertainty budget are frozen;
5. task functional unit and acceptance predicate are frozen;
6. requalification actions and evidence gaps are linked to #5488;
7. A→B receipt and accepted-output accounting are pinned or formally marked unresolved;
8. experiment, exclusions, analysis and stop rules are preregistered.

## 9. Claim ceiling

This document is an experimental design. It does not select a specific robot, certify a vendor product, establish standards conformity, authorize actuation, prove safety, establish physical interchangeability, show productivity improvement or claim superiority over industry. No physical test result exists in this tranche.
