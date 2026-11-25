# AERO Design-for-Inspection and Sensing Integrity Benchmark v0.1

**Owner:** Symtropy AERO inspection / observability track #4165  
**Composes:** AERO material evidence #7319 / #7342, physical pilot #7396, AP242/QIF interop #7395, Mycelix AeroCommons #6347, Symthaea uncertainty #7412 and evidence-action selector #13112  
**Status:** benchmark protocol only; no physical NDE/POD, SHM, or airworthiness result  
**Claim ceiling:** synthetic challenge semantics and future physically correlated design/inspection evidence

## 1. Research objective

Move inspection from a late maintenance check to a design-time property of the aircraft, manufactured detail and evidence plan.

NASA Langley's Design-for-Inspection (DFI) work proposes an automation-friendly inspectability metric using simulation tests to interrogate defect findability and sensitivity to variability while covering parameter space efficiently. It explicitly frames inspectability as a metric that can enter multi-objective design optimization alongside mass and manufacturability. This is a useful external methodology comparator, not a result that Luminous has reproduced.

The second problem is sensing integrity. An anomalous structural-health signal can be caused by structural damage, sensor degradation, environment/operating-state variation, acquisition faults or unknown conditions. A system that "corrects" a sensor fault without preserving the original signal can accidentally erase a real damage signature.

The benchmark therefore asks:

1. Can a design be assessed for the inspectability of declared defect families before design freeze?
2. Can inspection methods be matched to flaw size/orientation/depth, access, geometry and material/process context?
3. Can SHM distinguish structural damage from sensor faults and environmental shifts while retaining raw observations?
4. Can inspection evidence be connected to the exact physical unit, configuration, NDE method, equipment, calibration, acquisition time and uncertainty?
5. Can evidence-directed design/testing reduce total verification burden without lowering a hard detection or engineering requirement?

## 2. Boundaries and ownership

- Symtropy #4165 owns aerospace inspection, NDE/SHM observability semantics, DFI/inspection design interfaces and its benchmark.
- Mycelix owns serialized physical-unit/build/inspect/test lineage and artifact digest/byte-length integrity.
- Symthaea uncertainty #7412 owns observation-level uncertainty and sample/dependence provenance.
- Symthaea BASALT/ASSURE own generic evidence freshness, observer coverage and qualification lifecycle.
- Symthaea #13112 owns bounded evidence-action selection.
- Competent laboratories, engineering authorities and aviation authorities own physical procedure approval, acceptance and certification decisions.

Do not build a second NDE database, artifact ledger, evidence graph, invalidation engine, qualification state machine, aircraft optimizer or certification authority.

## 3. DFI design contract

### 3.1 Defect and inspection context

Each inspectability claim must identify:
- physical component/detail and exact product configuration;
- material system, lot, process batch, manufacturing route, geometry and joint/layup context;
- defect family and physically meaningful descriptors (type, size, orientation, depth, location, morphology);
- inspection method, procedure revision, equipment/instrument, access and surface/curvature constraints;
- operator/automation mode and acquisition settings where they affect the method;
- calibration evidence, interval and measurement acquisition time;
- coverage, detection threshold, false-call behaviour, sizing error and uncertainty;
- repairability, allowable repair, inspection interval and maintenance access where relevant.

A single DFI score must not hide defect families or inspection conditions. A design may be inspectable for one flaw class and poorly inspectable for another.

### 3.2 Design variables to co-optimize

Where the physics and requirements support it, expose:
- access envelope, probe/tool approach and removal/install clearance;
- skin/stringer/joint geometry, local thickness, curvature and layup;
- critical-region visibility, scan paths and coverage;
- sensor number/location/orientation, attachment and wiring/communication route;
- sensor redundancy, observability and maintainability;
- repair access, repair boundary and replacement strategy;
- manufacturing tolerances and process variability that change detectability.

Keep structural mass, strength, stiffness, buckling, fatigue/damage tolerance, aeroelastic response, manufacturability, energy/thermal effects and lifecycle maintenance as separate metrics/constraints. Hard structural or detection requirements are not tradable against a lighter design.

### 3.3 DFI simulation versus physical detection

Simulation-derived detectability and physical probability-of-detection evidence are different evidence classes. A simulation can identify difficult viewing angles, inaccessible surfaces or sensitivity to geometry/defect variation; it cannot on its own establish physical POD, procedure qualification or in-service detection performance.

Every DFI model must report:
- exact geometry/material/defect/inspection domain;
- solver/simulation version and input configuration;
- mesh/resolution and numerical convergence evidence;
- boundary/visibility/acoustic/thermal assumptions as applicable;
- uncertainty and model-form limitations;
- the physical tests still required to validate the prediction.

Out-of-domain geometry, defect morphology or inspection configuration must produce NOT_COVERED / UNKNOWN, not extrapolated confidence.

## 4. POD and NDE experiment contract

### 4.1 Conditional probability of detection

Treat POD as conditional on a declared examination configuration:

P(detect | flaw family, size, orientation, depth, location, geometry, material/process, procedure, equipment, calibration, operator, environment)

Do not report a universal POD number independent of its defect population and examination parameters.

For hit/miss data, ASTM E2862-23 provides a statistical analysis practice for hit/miss POD for a specific set of examination parameters. Select the method only when its assumptions and data type fit. Use the applicable governing standard, program requirement and authority-approved procedure; do not treat E2862 as a universal standard for every NDE modality or certification basis.

### 4.2 Experimental design

- Preregister the flaw families, sizes, orientations, depths, locations and blind truth labels.
- Include representative manufacturing/process defects and service damage only where their definitions and physical introduction are controlled.
- Split data by independent physical article, panel, lot, process batch, procedure, instrument and operator as applicable. Randomly splitting repeated indications from one panel is leakage.
- Preserve misses, false calls, ambiguous indications, censored measurements, excluded samples and invalid trials with reasons.
- Use blinded inspectors and independent flaw truth establishment where feasible.
- Select sample counts through the applicable method, precision/power and confidence requirements; do not invent a universal minimum N.
- Report POD curves/intervals only for the method and data type that support them; state the flaw size parameter, confidence method, excluded data and experimental-unit hierarchy.

### 4.3 Required metrics

Report separately:
- hit/miss counts and denominators by defect class;
- POD estimate and confidence bounds over the supported flaw-size range;
- false-call rate and sizing error;
- detection threshold and threshold uncertainty;
- coverage / inaccessible-surface fraction;
- operator/equipment/procedure variation;
- calibration failures and out-of-window observations;
- independent article/lot/batch/operator counts;
- inspection time, repeat scans, setup and access burden;
- repairability and maintenance downtime implications;
- explicit abstention and NOT_COVERED cases.

An a90/95 statement may be reported only when the applicable POD method and data justify it. It is not a universal acceptance threshold to apply to every aircraft, material or NDE method.

## 5. SHM sensing integrity benchmark

### 5.1 Competing hypotheses

For every changed signal, the evaluator must preserve and distinguish:
1. structural damage or degradation;
2. sensor / acquisition-chain fault;
3. environment or operating-state change;
4. unknown / out-of-domain condition.

Examples of sensor faults include bias, drift, gain change, saturation, dropout, stuck-at output, timing error, synchronization loss, cross-talk and intermittent connection. Challenge them both independently and in combination with real/injected structural damage.

### 5.2 Immutable observation lineage

- Raw sample bytes and raw acquisition metadata are immutable.
- Calibration, sensor identity, timebase, clock quality and acquisition configuration are pinned.
- A corrected/reconstructed signal is a new derived artifact with a parent reference, algorithm/model version, parameters, uncertainty and reason.
- Sensor reliability is a separate claim from the reconstructed signal.
- If the system cannot separate damage from sensor fault or environmental shift, it must abstain and request independent inspection/NDE.
- A correction must never overwrite raw data, clear a damage indication, alter the original detection label or promote a claim without an independent verification step.

### 5.3 SHM metrics

Report:
- structural-damage sensitivity/recall by damage family and severity;
- false alarms per operating hour or mission, with exposure denominator;
- sensor-fault detection/diagnosis precision and recall;
- damage-versus-sensor-fault confusion matrix;
- missed-damage rate when a sensor fault is simultaneous;
- probability calibration / reliability of sensor-health estimates;
- signal reconstruction error and uncertainty versus immutable raw signal;
- OOD/abstention and unresolved-cause rate;
- robustness to environment/temperature/load/aging and acquisition-clock drift;
- latency, power, compute, memory, sensor mass and maintenance burden.

Accuracy alone is insufficient. Report calibration and false-negative risk separately from reconstruction quality and sensor-health classification.

## 6. Frozen benchmark arms

For the same design and evidence corpus, compare:

1. Inspection-naive structural design baseline.
2. Conventional design-for-manufacture / maintenance review without DFI metric.
3. NASA-style simulation-based inspectability metric where its published scope and input assumptions match.
4. Fixed/manual sensor-layout baseline.
5. Physics-only SHM detector with no sensing-layer diagnosis.
6. Damage + sensor-fault competing-hypothesis model.
7. Full co-design and evidence layer with DFI, POD evidence, sensor reliability and owner-native provenance/freshness.

For test/inspection selection, compare:
- random / conventional inspection plan;
- space-filling design;
- highest model uncertainty;
- sensitivity-only;
- prediction disagreement;
- calibrated EVSI / expected decision-loss reduction;
- distribution-free minimax regret;
- finite-horizon policy from the existing evidence-action selector;
- qualified human engineering/NDE plan.

All arms use the same safety obligations, defect truth, data budget, inspection resources, hardware configuration and scoring code.

## 7. Prospective gates

These are research targets, not achieved results, industry standards or certification criteria:

- zero false acceptance of known wrong-unit, wrong-configuration, stale-calibration and missing-POD-domain controls on the bounded challenge corpus;
- 100% preservation of raw observation identity and provenance through the pipeline;
- zero silent deletion of missed, censored, excluded, false-call or invalid inspection records;
- no positive POD/SHM promotion outside the validated defect/material/geometry/procedure/environment domain;
- 100% deterministic semantic reports for deterministic inputs;
- explicit damage / sensor fault / environment / unknown disposition or abstention;
- any verification-effort reduction demonstrated only at matched defect coverage, detection confidence, false-negative constraints and hard engineering requirements.

Physical claims require blind, rights-cleared, independently reviewed evidence from representative articles and qualified procedures. Synthetic simulation and injected sensor faults can validate software/control logic, not physical POD or aircraft SHM acceptance.

## 8. Reference comparators

- NASA Langley Design for Inspection: NTRS 20240003953 proposes a simulation-based, automation-friendly inspectability metric evaluating defect findability and sensitivity to variability.
- NASA Langley / ASNT Inspectability Metric: NTRS 20240007593 describes integration of inspectability into multi-objective design optimization alongside mass/manufacturability.
- ASTM E2862-23: hit/miss POD analysis for a specific examination parameter set; not a universal NDE modality standard.
- MIL-HDBK-1823A: active DoD handbook for NDE system reliability assessment; DLA identifies it as guidance only, not a standalone requirement.
- Di Fiore et al. (8 June 2026), “Real-time sensing digital twin for aerospace structural health monitoring,” Structural and Multidisciplinary Optimization 69, article 159. It separates structural and sensing degradation and reconstructs sensor-fault-affected signals; its demonstration is a simulated composite wing panel and must not be represented as a physical flight/aircraft qualification.

## 9. Non-claims

This benchmark does not establish physical POD, structural health monitoring acceptance, fatigue/damage tolerance, maintenance interval, repair approval, structural substantiation, part acceptance, airworthiness, aircraft certification or authority to operate. It is an experiment and software-benchmark contract only.
