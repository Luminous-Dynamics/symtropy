# Construction Robot Round-Robin Measurement Protocol V0.1

**Issue:** [#3591 — cross-laboratory reproducibility and vendor-neutral measurement](https://github.com/Luminous-Dynamics/symtropy/issues/3591)  
**Study profile:** `construction-layout-roundrobin-v0.1.json`  
**Schema:** `construction-roundrobin-study-plan-v0.1.schema.json`  
**Status:** proposed research protocol; no physical study has been executed.

## Purpose

The benchmark must validate its own measurement process. A robot result that repeats in one laboratory may not reproduce when the instrument, calibration chain, operator, site, environment or test organization changes.

This protocol is intended to estimate, separately:

1. robot/task performance variability;
2. reference measurement-method repeatability and reproducibility;
3. between-laboratory and between-session effects;
4. platform, serial-unit, operator, site, configuration and ODD effects;
5. uncertainty and the limits of the claims supported by the collected evidence.

This is a study-design layer. It is not a robot controller, safety case, certification, regulatory conformity assessment or proof of vendor superiority.

## Standards and research foundation

Use existing measurement science rather than create a competing standards framework:

- [NIST Mobility Performance of Robotic Systems](https://www.nist.gov/programs-projects/mobility-performance-robotic-systems): mobile-robot metrology, reference measurements, testbeds and dynamic-environment performance methods.
- [AMRA-201:2026](https://amr-alliance.org/amra-201/): general mobile-robot requirements and test methods for solid travel surfaces, where the platform and scope fit.
- [ISO 5725-2:2025](https://www.iso.org/standard/90054.html): collaborative interlaboratory estimation of repeatability and reproducibility for continuous-scale measurement methods.
- [ASTM E691-23](https://store.astm.org/standards/e691): interlaboratory study planning, conduct, consistency diagnostics and precision estimates.
- [NIOSH construction robotics guidance, 7 October 2026](https://www.cdc.gov/niosh/bulletin/2026/safety-in-construction-robotics.html): construction-specific context and worksite change.

**Applicability warning:** ISO 5725-2 and ASTM E691 are foundations for measurement-method precision studies, not turnkey statistical models for every robot outcome. Their continuous single-result model must not be indiscriminately applied to event counts, binary acceptance, failures, time-to-event or crew productivity. The protocol therefore uses endpoint-appropriate analysis and labels any adaptation. It must not claim conformance unless the complete applicable method is followed.

## Initial study: autonomous construction layout

Start with layout/printing because a frozen model and task can be distributed, completed geometry can be independently surveyed, and initial work can take place on a protected test slab without introducing powered-tool or earthmoving hazards.

Use two distinct tracks:

### 1. Vendor-configuration reproduction

Reproduce a named manufacturer's published setup as closely as possible. Bind the exact robot model and serial unit, software/firmware, mode, payload/tool, survey instrument, control configuration, stationing, range, environment and documented limitations.

Current published references include:

- HP SitePrint: [official specifications](https://www.hp.com/us-en/printers/site-print/layout-robot.html). The published ±2 mm layout tolerance and average ±0.8 mm floor-level accuracy are conditional on the stated high-accuracy configuration, including the SMR prism, High Accuracy Print Mode and a 1-inch total station at 5–30 m.
- Dusty FieldPrinter 2: [manufacturer technical specification](https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs). The published ±1.6 mm print-accuracy statement is conditional on accurate surveyed control.

These remain `vendor_reported` until independently reproduced. Do not silently replace a published condition with a convenient local setup.

### 2. Common independent referee

Run platforms on identical frozen task inputs and task strata. Use independent reference instruments and checkpoints that are not used by the robot for localization, control or navigation.

The vendor-reproduction and common-referee tracks answer different questions. Never pool them as though they were identical experimental conditions.

## Two different reference artifacts

Develop and identify two distinct artifacts:

1. **Measurement-chain check artifact.** A stable, independently characterized reference geometry or artifact used to check each laboratory's measurement chain before and after a session. Its calibration, custody, environmental sensitivity, damage inspection and uncertainty must be documented.
2. **Construction task challenge.** A frozen work package and physical task surface on which the robot performs the task. Bind the approved design/model revision, export, coordinate frame, worksite snapshot, geometry digest, feature inventory and acceptance rules.

The check artifact is not a substitute for a real slab or site task. Passing the artifact check does not establish that the robot can perform the task, or that the task surface represents the full operating domain.

## Study design

### Experimental factors and identifiability

Record at least the following factors when relevant:

- laboratory and physical site;
- platform model and exact serial unit;
- software/firmware, configuration, payload and tool;
- operator and operator-familiarization state;
- reference instrument, calibration certificate/interval and survey network;
- session, day, order and environmental conditions;
- task stratum, geometry revision and ODD.

Prefer crossed designs where feasible. If serial units, platforms, instruments or operators cannot be crossed across laboratories, document the nesting and the resulting non-identifiability. Do not attribute a lab/unit/operator effect to the robot model when the factors are confounded.

A complete crossed design is not always operationally possible. A balanced or unbalanced incomplete-block design may be acceptable, but the estimand, missing cells, model assumptions and transfer limitations must be declared before confirmatory analysis.

### Repeatability and reproducibility

Estimate separately where the study design supports it:

- within-laboratory repeatability under stable operator/apparatus/session conditions;
- within-laboratory intermediate precision across days, operators or setups;
- between-laboratory reproducibility;
- reference-method trueness/bias against an accepted reference value;
- between-serial-unit variability within each platform;
- task/ODD and platform-by-condition effects.

These are not interchangeable measures. A repeatable robot output can be biased. A reproducible measurement method does not establish that the robot is safe.

### Reference-chain checks

For every laboratory session:

- measure the common check artifact before and after the session;
- record the reference instrument identity, model, serial, calibration authority, certificate digest, validity interval, configuration and uncertainty;
- record the control network, adjustment, coordinate frame, transform resources and their exact revisions;
- document the instrument's independence from robot control and navigation;
- identify shared calibration providers, reference clocks, software, survey networks, staff, datasets and other possible common-mode dependencies;
- preserve raw observations and derived artifacts separately.

A digest establishes byte identity, not the truth of a calibration certificate or the physical accuracy of an instrument. Independence must be supported by dependency evidence; different organization names are insufficient.

## Measurement models, uncertainty budgets and decision rules

### Measurands before instruments

Freeze each measurand before selecting the instrument or calculating a score. Examples include signed X/Y coordinate residual in the approved target frame, radial plan-position residual, signed height residual, baseline length error, orientation error, feature completeness and task-unit acceptance. Define the measurand, units, target frame/vertical datum, coordinate epoch, evaluation location, sampling unit and sign convention. Do not conflate a robot's internal estimate with the referee's measurement of the physical output.

### Uncertainty budget

For each continuous measurement, retain a documented measurement model and identify corrections and uncertainty components. Relevant layout contributors may include:
- control-network adjustment and point-coordinate covariance;
- reference-instrument calibration and field precision;
- centering, prism/reflector constants and target geometry;
- instrument setup, leveling, station resection and orientation;
- angular/distance observations and atmospheric corrections;
- environmental change and thermal expansion of reference artifacts;
- coordinate-frame transformation resources, datum/epoch and grid versions;
- physical feature extraction, target-centre definition and repeatability;
- time synchronization and any interpolation or motion compensation;
- common-mode dependencies and correlations among inputs.

Classify uncertainty inputs as Type A (statistical) or Type B (other documented information), apply significant known corrections, retain covariance/correlation where components share a source, and report combined standard uncertainty and expanded uncertainty with its coverage factor. Do not simply add all instrument catalogue accuracy figures, treat correlated terms as independent, or hide a known bias by inflating uncertainty.

Use the current JCGM/BIPM GUM guidance as the metrological foundation. The BIPM catalogue currently lists JCGM 100:2008 Amendment 1:2026 on nonlinearity in measurement models, GUMsuite:2026, GUM-5:2026 examples, GUM-6:2020 on measurement models, JCGM 101:2008 for Monte Carlo propagation, JCGM 102:2011 for multiple output quantities and JCGM 106:2012 on uncertainty in conformity assessment. Assess nonlinear propagation explicitly for coordinate transformations, range/angle-to-coordinate conversion, radial error, geometric fitting and derived task metrics. Use a documented Monte Carlo method when linearization or distribution assumptions are not adequate; retain the input distributions, correlations, sample count, algorithm and convergence diagnostics.

References:
- BIPM/JCGM current guide catalogue: https://www.bipm.org/en/web/guest/publications/guides
- JCGM 100:2008 (GUM): https://www.bipm.org/en/doi/10.59161/jcgm100-2008e
- NIST TN 1297, combined standard uncertainty: https://www.nist.gov/pml/nist-technical-note-1297/nist-tn-1297-5-combined-standard-uncertainty
- NIST TN 1297, expanded uncertainty: https://www.nist.gov/pml/nist-technical-note-1297/nist-tn-1297-6-expanded-uncertainty
- NIST TN 1297, reporting: https://www.nist.gov/pml/nist-technical-note-1297/nist-tn-1297-7-reporting-uncertainty

### Decision rules and guard bands

A measurement result and a conformance decision are different outputs. Before confirmatory data collection, freeze the task-specific acceptance limit, the decision rule, the guard band and the risks of false acceptance and false rejection. For a two-sided layout tolerance, define both limits and state the rule for a result close to either boundary. Report the measured value, uncertainty and decision separately.

Use ILAC-G8:09/2019 as guidance for decision rules and statements of conformity where applicable. Its guard-band concept makes the acceptance limit more restrictive than the specification limit to manage decision risk. The appropriate rule depends on the cost and consequences of false acceptance and false rejection; do not import one universal guard-band width or claim that a zero guard band is risk-free.

For this benchmark, the evaluator should be able to return:
- `accepted_under_named_rule`;
- `rejected_under_named_rule`;
- `indeterminate_measurement_capability_or_boundary`;
- `not_evaluated`.

An uncertainty interval that overlaps the rejection region must not be silently reported as an unconditional pass. If reference uncertainty cannot resolve the task-specific decision margin, the result is indeterminate for that claim, even when the point estimate lies inside tolerance.

References:
- ILAC G8:09/2019, decision rules and statements of conformity: https://ilac.org/publications-and-resources/ilac-guidance-series/
- ISO 17123-5:2018, total-station field precision procedures: https://www.iso.org/standard/71689.html

ISO 17123-5 is useful for field verification of total-station coordinate-measurement precision/repeatability. It is not a comprehensive acceptance or performance evaluation of the robot, the full survey network or the entire reference chain. Use it within its actual scope, then retain the additional task-specific control-network, frame, covariance and referee checks required by this protocol.

### Round-robin comparability

A cross-laboratory study must compare both reported values and uncertainty/decision-rule implementations. Two laboratories that report similar millimetre residuals but use materially different uncertainty models or acceptance rules have not demonstrated equivalent measurement procedures. Preserve each laboratory's raw observations, corrections, covariance information, uncertainty budget, coverage factor, decision rule and guard band so disagreements can be diagnosed rather than averaged away.

## Randomization, blinding and preregistration

Before confirmatory collection, freeze:

- primary endpoints and task strata;
- hypotheses and practically meaningful decision differences;
- the independent experimental unit for each endpoint;
- sample-size and precision rationale;
- randomization and balancing plan;
- exclusions, invalid-run handling and stopping rules;
- analysis models, multiplicity handling and missing-data treatment;
- challenge custody and reveal procedure.

Use a separate pilot to estimate variance and method feasibility. Do not use the same exploratory pilot as confirmatory evidence unless that reuse is explicitly justified and disclosed.

Randomize platform, scenario and run order within safe operational constraints. Separate familiarization from scored runs. Retain every attempted run, including interruptions, operator interventions, aborted tasks, invalid setups, missing data and protocol deviations. Use blinded run identifiers and evaluator-side challenge custody where practical.

## Endpoint and statistical design

### Continuous metrology

Report signed residuals and bias, absolute residual distributions, repeatability and reproducibility standard deviations, reference uncertainty, uncertainty intervals, p50/p95/p99 and observed maxima by task/ODD stratum.

Do not use p99 as a worst-case guarantee. Report the sample count, independent-run count, clustering, raw observations and confidence/uncertainty interval. A checkpoint or printed vertex is not automatically an independent trial.

### Task quality and completeness

Report critical feature omissions, duplicates, dimensional/positional defects, first-pass acceptance, rework and independently accepted output. Do not substitute a global average residual for critical-feature completeness.

### Productivity and reliability

Report separately:

- accepted output per robot-hour;
- accepted output per total crew-hour;
- elapsed time per accepted output;
- setup, stationing, coordination, survey, VDC/BIM preparation, operator/spotter, QA, rework, interruption and recovery;
- availability, completion, aborts and reason-coded failed runs;
- energy and consumables where relevant.

Print speed is not accepted project output. Robot-only time is not total workflow time.

### Model choice and sample size

Use models appropriate to each endpoint. Hierarchical/mixed-effects models may separate platform, serial unit, laboratory, operator, session, task and condition effects when the design makes them identifiable. Binary acceptance, defect counts, event rates, durations and resource ratios require suitable endpoint-specific models. Do not force every outcome into a normal model or a single score.

Determine confirmatory sample size from pilot variance, desired uncertainty/precision, the decision margin and a predeclared practically meaningful difference. Report the design assumptions and uncertainty. Do not choose sample size after observing which result is significant.

## Safety and performance remain separate

Safety qualification remains under the physical protocol and safety-specific owners. The round-robin performance study may record safety-track references and test-condition boundaries, but it cannot issue or upgrade a safety qualification.

No productivity, accuracy or reliability improvement compensates for a failed, unknown or untested safety requirement. No overall safety/productivity score is permitted.

## Data and evidence requirements

Every scored run should bind:

- study/profile and preregistration digests;
- lab/site/session and blinded run ID;
- exact platform, serial unit, software, firmware, tool, payload and configuration;
- exact task/design/export/frame/site epoch and ODD;
- reference instruments, calibration intervals, artifact checks and uncertainty;
- raw measurement/evidence artifact digests;
- independent unit and denominator definitions;
- deviations, interventions, invalid/aborted disposition and reason codes;
- endpoint results, analysis version and claim scope;
- independent adjudication and custody references.

Reuse the existing trial-receipt and claim-admissibility owners. Do not create another evidence ledger or survey engine.

## Initial protocol-quality gates

These are targets for study completeness, not robot performance claims:

- 100% of scored runs bind exact task, platform/configuration and ODD identity;
- 100% of reference measurements bind instrument, calibration interval, reference artifact and uncertainty;
- 100% of attempted runs reconcile to included, invalid, aborted or missing-with-reason dispositions;
- no unqualified pooling across labs, units, platforms, tracks or task strata;
- repeatability/reproducibility estimates disclose uncertainty and variance-component limitations;
- a cross-site transfer claim requires independent reproduction at a second site/laboratory and a declared configuration scope;
- no performance evidence upgrades safety qualification.

## Existing ownership boundaries

- #3523/#3524: task-matched layout benchmark and common referee.
- #3505/#3506: physical ODD and safety/metrology protocol.
- #3521/#3522: physical-trial receipt.
- #1607: survey control, coordinate frames, adjustment, calibration and uncertainty.
- #1300: blind challenge commitment and custody.
- #3531: claim-specific admissibility.
- #3542/#3558: dynamic worksite epochs and observation coverage.
- #3566: end-to-end permission-to-evidence handoff conformance.
- Symthaea #4918: claim-relative dependency lineage and common-mode independence.

This protocol owns interlaboratory study design and reproducibility analysis only. It must not create a competing survey engine, safety controller, scenario runtime, event store or evidence authority.
