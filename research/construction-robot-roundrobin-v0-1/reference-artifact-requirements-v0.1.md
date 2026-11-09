# Construction Layout Round-Robin — Reference Artifact Requirements V0.1

**Parent:** [#3591 — cross-laboratory reproducibility and vendor-neutral measurement](https://github.com/Luminous-Dynamics/symtropy/issues/3591)  
**Protocol:** [README.md](README.md)  
**Status:** requirements proposal only; no physical artifact has been designed, calibrated or built.

## 1. Purpose and strict boundary

The round-robin study needs a portable reference artifact that helps determine whether different laboratories' measurement chains produce mutually consistent results.

The artifact is a **measurement-chain transfer check**, not:
- a robot navigation or control target;
- a substitute for the independent survey network at the physical task site;
- a safety test or sensor-coverage proof;
- a full construction task;
- an assertion that the artifact's assigned values are exact or uncertainty-free.

Maintain two independent objects:

1. **Transfer artifact:** checks the laboratory's reference measurement method before and after a session.
2. **Physical task challenge:** tests the robot on a frozen construction work package and independently surveyed task surface.

A passing transfer-artifact check does not prove task success. A good task result does not prove that the laboratory's measurement chain is stable.

## 2. Artifact design requirements

The physical design must be selected only after metrology and handling feasibility are reviewed. V0.1 deliberately does not prescribe a material, dimension, point count or universal tolerance.

The design proposal should evaluate:

- **Spatial distribution:** reference features near corners, edges, central regions and intermediate locations; avoid a geometry in which all information lies on one line or in one compact cluster.
- **Multiple baselines:** short and long distances, diagonal distances and differently oriented baselines to expose scale, rotation, axis and anisotropic effects.
- **Angular geometry:** orthogonal and non-orthogonal feature relationships to expose orientation and squareness errors.
- **Feature classes:** stable point centres and line/edge features; consider arcs or other geometry only where the reference method can characterize them with adequate uncertainty.
- **Planarity and height:** characterize plane departure and any deliberately incorporated height steps separately from XY position.
- **Accessibility:** permit the selected independent instruments to observe the required features without relying on robot sensors or robot control hardware.
- **Reorientation:** where the design allows it, measure the artifact in multiple orientations and placements to reveal fixture, alignment and axis-dependent effects.
- **Durability:** resist ordinary transport, handling and cleaning without changing assigned geometry beyond the declared stability envelope.
- **Environmental sensitivity:** characterize relevant temperature, humidity, mounting, support, vibration and thermal-gradient effects. Record correction models and applicability limits where needed.
- **Unique identity:** permanently identify the physical artifact, revision, feature map and custody seals or tamper-evident controls appropriate to the study.

Do not make the artifact so specialized that only one vendor's laser tracker, prism, fiducial or data format can measure it. Vendor-specific reproduction belongs in the separate vendor-configuration track.

## 3. Assigned reference values and uncertainty

Each artifact revision must have a controlled assignment record with:

- physical artifact ID, design revision and material/assembly identity;
- canonical feature IDs and feature definitions;
- assigned coordinates, distances, angles, plane/height values and units, as applicable;
- coordinate-frame definition, axis convention and orientation;
- calibration/characterization method, laboratory and date;
- reference instrument identities and calibration evidence;
- uncertainty budget and covariance/correlation information where relevant;
- environmental conditions and correction model;
- traceability chain and responsible authority;
- validity interval, stability limits and recalibration/rejection triggers;
- raw characterization artifacts and their content digests;
- independent review/adjudication and known limitations.

An assigned value is not uncertainty-free ground truth. Its uncertainty must be included in the comparison with a laboratory's measurement result. If reference uncertainty is too large to resolve the study's decision margin, the artifact is unsuitable for that endpoint.

A string digest establishes byte identity; it does not authenticate the calibration authority or prove that the physical artifact still matches its assigned values.

## 4. Before/after session procedure

Every participating laboratory should:

1. inspect and identify the artifact on receipt;
2. record transport, custody, seal, damage and environmental history;
3. stabilize the artifact under the declared measurement conditions;
4. perform the frozen measurement procedure using the independent reference chain;
5. retain raw observations, operator, instrument, configuration, timestamp/timebase, environmental records and derived results;
6. repeat the same procedure after the robot task session;
7. compare before/after measurements with the assigned values and each other using the preregistered uncertainty-aware decision rule;
8. preserve out-of-limit, failed, missing and indeterminate checks rather than replacing them with a fresh best result.

The study plan must say whether the check is performed by the same or different operators, whether the artifact is repositioned, which orientations are used, and whether the session is repeatability or intermediate-precision data. These choices affect the estimand and must be frozen before confirmatory data collection.

## 5. Uncertainty-aware decision rules

Do not use a universal residual threshold for every instrument, artifact or task.

For each measured quantity:
- state the measurand and unit;
- define the assigned value and its uncertainty;
- define the laboratory result and its uncertainty;
- identify correlated uncertainty components and common reference dependencies;
- preregister the decision rule and guard band against the task-specific decision margin;
- report signed difference, uncertainty and disposition separately.

A check is not a pass merely because the point estimate lies inside a tolerance. A check is not a fail merely because two rounded values differ. The comparison must account for both uncertainty budgets and the declared decision rule.

The analysis must distinguish:
- stable agreement with the assigned value;
- repeatable but biased results;
- excessive within-laboratory variation;
- between-session drift;
- laboratory-specific measurement differences;
- an artifact whose own stability or custody is no longer demonstrated.

If before/after checks indicate unacceptable drift, the study protocol must define which session results become invalid, indeterminate or subject to remeasurement. Do not silently delete the session or infer that the robot caused the discrepancy.

## 6. Common-mode independence

The artifact characterization and the participating laboratory's check should document dependencies on:
- reference instruments and their calibration providers;
- coordinate-control networks and transformation resources;
- operators, surveyors and assessors;
- firmware, processing software and data-conversion tools;
- reference clocks and synchronization services;
- upstream data or prior artifact characterizations.

Use Symthaea #4918 for claim-relative dependency lineage. Do not create another generic dependency graph in this artifact package.

Different instrument serial numbers or different laboratory names do not automatically establish independent error sources.

## 7. Round-robin handling and study design

The study coordinator should freeze:
- artifact revision and feature map;
- transport container, packing, orientation and handling instructions;
- receipt/return inspection and custody events;
- allowed environmental range and equilibration procedure;
- measurement procedure and supported instruments;
- operator qualification/familiarization requirements;
- randomization of artifact orientation, station and run order;
- before/after check schedule;
- invalidity and recalibration rules;
- raw-data submission format, blind run ID and evaluator reveal process.

Where feasible, send the same physical artifact through participating laboratories in randomized order and include a return-to-origin check. A return check can help reveal drift during transport or a sequence effect, but cannot by itself attribute the cause. Consider a duplicate artifact only if its equivalence and between-artifact variability are separately characterized.

A transfer artifact is a travelling measurement standard for the study, not a replacement for each laboratory's own traceability chain.

## 8. Metrics to report

Report separately by feature class, orientation, laboratory, instrument, operator and session where data permit:

- signed feature-coordinate differences;
- baseline-length and angular differences;
- plane/height deviations;
- within-laboratory repeatability;
- intermediate precision across days/operators/setup;
- between-laboratory reproducibility;
- before/after drift;
- artifact stability and custody exceptions;
- combined uncertainty and decision-rule disposition.

Do not collapse these into one artifact score. A laboratory may be repeatable but biased, or accurate in one orientation while showing anisotropic error in another.

## 9. Open engineering decisions before physical build

The study lead and metrology partner must decide, and record why:

1. artifact geometry, materials, dimensions and mass;
2. feature types compatible with all selected referee instruments;
3. calibration laboratory and traceability route;
4. allowed transport and environmental envelope;
5. assigned uncertainty and stability requirements relative to the intended layout decision margin;
6. whether one artifact can serve all participating laboratories without introducing incompatible fixtures;
7. artifact inspection, recalibration and retirement criteria;
8. cost, portability and custody responsibilities.

These are unresolved design inputs. Do not fill them with arbitrary values simply to make the profile look complete.

## 10. Acceptance boundary

A laboratory's reference-chain result may support only the measurement-method claims and session-validity disposition named in the study plan.

It does not establish:
- robot positional accuracy;
- task feature completeness;
- safety or hazard-detection performance;
- field reliability;
- cross-site transfer;
- manufacturer superiority.

Those claims require their own task-specific, independently measured evidence.
