# JointLab — Instrumented Rotary Joint Physical Qualification Protocol

Status: proposed bench and measurement contract. No physical rig, actuator prototype, calibrated instrument set, human-contact validation or measured performance result is claimed.

Owner: [HUMANOID-JOINT-CARTRIDGE-001 #4337](https://github.com/Luminous-Dynamics/symtropy/issues/4337)

Composes with:
- [#4557 rotational inertia qualification](https://github.com/Luminous-Dynamics/symtropy/issues/4557)
- [#4179 actuator loss-map qualification](https://github.com/Luminous-Dynamics/symtropy/issues/4179)
- [#4044 energy accounting](https://github.com/Luminous-Dynamics/symtropy/issues/4044)
- [#4800 JointLab experiment definition](https://github.com/Luminous-Dynamics/symtropy/issues/4800)
- [#4801 system identification](https://github.com/Luminous-Dynamics/symtropy/issues/4801)
- [#4802 simulation-to-real discrepancy](https://github.com/Luminous-Dynamics/symtropy/issues/4802)
- [Sol-Atlas interface geometry owner #708](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/708)
- [Mycelix serialized article lifecycle crosswalk #6053](https://github.com/Luminous-Dynamics/mycelix/pull/6053)

This protocol does not duplicate the owners' schemas, ledgers or authority. It defines the physical measurement and qualification sequence.

## 1. Purpose

Turn one rotary-joint cartridge into a measurable engineering reference article. The same exact physical configuration must be traceable through geometry, mass properties, actuator maps, firmware, calibration, simulation, bench measurements, repair and requalification.

The first milestone is not a humanoid robot. It is one rotary joint whose commanded torque, measured torque, angular response, electrical input, mechanical output, thermal state, friction, backlash and post-service state can be independently reconciled.

## 2. Qualification article identity

Every run must identify an immutable configuration generation containing:

- serial number and article revision;
- CAD source revision and neutral export digest (STEP/AP242 as appropriate);
- body/link geometry, centre of mass, density/material assignment and validated inertia operator;
- motor, inverter/drive, gearbox, bearing, seal, encoder and temperature-sensor manufacturer/model/serial/lot;
- gear ratio, transmission arrangement, lubricant, preload and assembly torque;
- firmware, controller, calibration, safety limits and actuator loss-map digests;
- power supply/battery, voltage/current operating envelope and auxiliary devices;
- fixture, adapter, load, boundary conditions and instrument identities.

A repair or replacement of a motor, gearbox, bearing, encoder, controller, link, toolhead, seal, fastener with a structural role, material or firmware creates a new configuration generation. Qualification evidence is inherited only where an explicit impact analysis proves that the relevant requirement and evidence are unaffected.

Mass/inertia provenance is mandatory: a CAD mesh is not a mass-property certificate. Density, material assignment, centre-of-mass convention, coordinate frame, mesh/solid revision and numerical integration method must be retained.

## 3. Proposed bench architecture

The bench must separate actuator-side measurements from output-joint measurements.

### Mechanical path

- Rigid guarded base and reproducible joint mounting interface.
- Calibrated inline output torque transducer at the declared joint boundary.
- Independent output-angle encoder or angular metrology reference.
- Programmable dynamometer or servo load capable of absorbing and, if in scope, driving torque in all four torque/speed quadrants.
- Replaceable inertial/load fixtures representing declared link and payload envelopes.
- Reaction-torque and fixture-deflection measurement where fixture compliance is material.
- Independent emergency stop, physical guarding and containment appropriate to stored energy and rotating parts.

### Electrical path

- Isolated, calibrated DC-bus voltage and current measurement at the declared input boundary.
- Drive/motor phase-current measurement where required for loss separation.
- Synchronized power analyser or sampled V/I recorder with stated bandwidth and uncertainty.
- Auxiliary power measurement for control electronics, pumps, fans and other non-actuator loads.
- Interlocked power isolation and discharge path.

### Thermal and condition path

- Temperature sensors at motor winding or validated winding estimator, motor case, drive, gearbox, bearings and ambient.
- Vibration sensors at bearing/gearbox locations where required by the fault model.
- Encoder/drive health, current limit, fault, watchdog and thermal-derating telemetry.
- Optional acoustic, lubricant-debris or seal-condition observations where the use case requires them.

### Data-acquisition path

All channels share a qualified time base and carry:
- source clock identity and epoch;
- sample sequence and source timestamp;
- calibrated instrument ID and calibration due date;
- units, scale, range and uncertainty;
- missing-data, saturation and filter state;
- raw-file digest and acquisition-software version.

A recently received buffered packet is not evidence of a recent physical observation. Source-clock freshness must be qualified independently from host receive time.

## 4. Calibration and metrology gate

Before any performance test:

1. Verify instrument serials, calibration certificates, traceability chain, range and due dates.
2. Perform zero, span and polarity checks on torque, angle, voltage, current and temperature channels.
3. Verify encoder direction and the sign convention for positive mechanical power.
4. Measure fixture compliance and parasitic friction with the actuator disconnected or independently characterized.
5. Verify channel time alignment using a common physical trigger.
6. Establish a measurement uncertainty budget for torque, speed, electrical power, integrated energy and temperature.
7. Run a repeated reference condition to quantify short-term repeatability and instrument drift.

No result is accepted if the measurement uncertainty is too large to distinguish the observed value from the pre-registered acceptance limit. Do not subtract unexplained fixture losses as if they were measured actuator losses.

## 5. Test sequence

The exact numeric envelope must be frozen by the named joint-location/use-case owner before hardware selection. The sequence below defines what to measure, not an invented motor rating.

### A. Static and low-speed mechanics

- torque-angle curve in both directions;
- breakaway torque and running friction;
- holding torque and drift;
- torsional stiffness and compliance;
- backlash, lost motion and hysteresis;
- repeatability after direction reversal;
- bearing and fixture parasitic torque.

### B. Dynamic torque-speed envelope

Sample the agreed torque-speed operating domain in all four quadrants:
- positive torque / positive speed;
- positive torque / negative speed;
- negative torque / positive speed;
- negative torque / negative speed.

At each declared point, record torque, speed, bus voltage/current, drive state, temperature and uncertainty. Keep acceleration ramps and dwell intervals consistent. Mark cells that are unmeasured, extrapolated, thermally derated or rejected; never fill them with a guessed efficiency.

### C. Closed-loop tracking and impedance

- step and ramp tracking;
- sinusoidal/chirp response over the declared bandwidth;
- position, velocity and torque tracking error;
- phase lag and overshoot;
- backdrivability and reflected inertia;
- commanded impedance/admittance under representative loads;
- saturation, anti-windup and recovery behaviour.

### D. Electrical and mechanical energy

Measure separately:
- DC-bus input energy;
- motor/drive electrical energy where instrumented;
- mechanical output work at the joint;
- positive mechanical work and negative mechanical work;
- auxiliary energy;
- energy residual and measurement uncertainty.

For measured joint torque \(\tau(t)\) and angular velocity \(\omega(t)\):

\[
P_{\mathrm{mech}}(t)=\tau(t)\,\omega(t)
\]

\[
W_{\mathrm{mech}}=\int_{t_0}^{t_1}P_{\mathrm{mech}}(t)\,dt
\]

Negative mechanical work is not proof of battery regeneration. Regenerative electrical return must be measured at the declared electrical boundary with correct sign, timing and loss accounting. Do not report mechanical work as electrical input or battery savings.

### E. Thermal behaviour

- cold start, warm-up and steady-state operation;
- duty-cycle transitions and peak-load events;
- winding/case/drive/gearbox/bearing temperature gradients;
- thermal lag and derating onset;
- cool-down and restart behaviour;
- hot-spot estimation uncertainty.

### F. Durability and environmental protection

Only after the approved load spectrum, environment and hazard controls are frozen:
- representative duty-cycle fatigue;
- peak/overload cases within an approved safe test fixture;
- shock/vibration and repeated direction reversal;
- ingress/condensation/corrosion and cleaning exposure;
- seal wear and service/reseal cycle;
- post-test backlash, stiffness, efficiency, noise and calibration drift.

Do not infer IP rating from a sealed enclosure appearance. Ingress tests require the applicable standard method and a qualified test facility.

### G. Fault and recovery tests

- source encoder disagreement or dropout;
- motor/gearbox temperature sensor fault;
- stale or replayed source observation;
- controller/drive communication loss;
- watchdog timeout and emergency-stop;
- power loss while loaded;
- current/torque limit activation;
- restart after an unresolved fault;
- replacement of a sensor or actuator followed by calibration and requalification.

The local controller and safety supervisor remain authoritative. A planner, learned policy or remote service never bypasses a physical limit or emergency stop.

## 6. Simulation-to-physical correlation

For each test, freeze a matched simulation input:
- exact article configuration and mass-property digest;
- exact actuator map and controller/firmware digest;
- exact load, fixture, boundary conditions and initial state;
- same command trajectory, sample timing and operating envelope;
- solver version, integration method, step size and contact model.

Compare time histories, not only end-state values:
- torque;
- angle and angular velocity;
- current and voltage;
- power and integrated energy;
- thermal trajectories;
- backlash, hysteresis and stiffness;
- fault detection and stop response.

Report bias, MAE/RMSE, maximum residual, interval coverage, residual autocorrelation and operating-envelope strata. Separate calibration data from blind validation data; never fit and score on the same cycles.

Proposed internal target for the first demonstrator (aspirational, not measured): on blind validation conditions inside the qualified envelope, achieve no more than 10% normalized RMS error for torque/angle response and energy integrals, with calibrated 90% predictive intervals. Tighten the target only after the metrology repeatability and instrument uncertainty are established. A high average fit cannot conceal a failed thermal, peak-load or quadrant-specific case.

## 7. Model and benchmark scorecard

Keep each dimension independent:

| Dimension | Primary output | Required stratification |
|---|---|---|
| Torque production | torque error, peak/continuous envelope | speed, direction, temperature |
| Motion | position/speed error, bandwidth, overshoot | load, direction, bus voltage |
| Transmission | efficiency, backlash, stiffness, friction | torque, speed, thermal state |
| Energy | Wh in, Wh out, Wh auxiliary, residual | all four quadrants and duty-cycle |
| Thermal | temperature, time to derating, recovery | ambient, load, speed, duty cycle |
| Reliability | failures, cycle count, drift | configuration and failure mode |
| Safety | stop latency, stop distance, residual torque | fault, speed, load, pose |
| Maintainability | repair time, calibration time, retest scope | component replaced and revision |
| Model fidelity | residual and predictive interval coverage | blind operating-envelope strata |
| Lifecycle | cost per qualified hour/task, service burden | article generation and usage |

Do not create a composite score that allows good efficiency to conceal unsafe stopping, or low mass to conceal fatigue failure.

## 8. Baselines and comparison

Compare candidate joint architectures under the same load spectrum, fixture, metrology, control bandwidth and energy boundary. At minimum consider:
- frameless PMSM + planetary reduction;
- frameless PMSM + strain-wave reduction;
- cycloidal reduction where impact/load/service requirements justify it;
- direct drive where torque density, thermal and envelope permit.

Do not compare a bare motor data sheet with a complete gearbox-equipped joint. Publish the joint-level torque-speed map, mass, volume, energy, thermal limits, backlash, stiffness, fatigue and serviceability.

Commercial product/robot specifications are context only until independently measured under matched protocols. The aim is not a higher peak-torque headline; it is more usable torque-speed coverage per kilogram and watt, with lower thermal derating, longer life, safer contact and faster repair.

## 9. Initial acceptance gates

1. Approved use case, load spectrum, environment and hazard analysis.
2. Valid instrument calibrations, source-clock alignment and uncertainty budget.
3. Exact serialized configuration and mass-property provenance.
4. All four electrical/mechanical quadrants measured or explicitly marked outside scope.
5. No silent extrapolation outside measured/qualified operating cells.
6. Blind simulation-to-physical residual within the preregistered envelope.
7. Thermal, fatigue, ingress and fault tests passed independently; no surrogate evidence.
8. Repair/reseal/replacement creates a new configuration generation and triggers the scoped retest.
9. Independent evidence package contains raw data, calibration certificates, analysis code/version, uncertainty, result digests and exact article revision.

The acceptance values must be approved against the intended application. This document does not declare a specific joint safe for human contact or ready for production.

## 10. Standards and regulatory mapping

The selected product/use case determines applicability:
- industrial robot component and integration: assess ISO 10218-1:2025 and ISO 10218-2:2025;
- service/personal-care humanoid: track ISO/FDIS 13482 Edition 2 (2026), which is still under development/approval;
- external limb prosthesis/orthosis: ISO 22523:2006 remains published/current while ISO/FDIS 22523 Edition 2 (2026) is in approval;
- medical rehabilitation/assessment robot: IEC 80601-2-78:2019 with Amendment 1:2024 is published; a second edition is under development. It explicitly excludes external limb prostheses;
- medical-device lifecycle risk management: ISO 14971:2019 remains current after review/confirmation in 2025;
- South Africa medical-device classification/licensing: assess SAHPRA's current Medical Device Classification Guideline SAHPGL-MD-04 Version 5 (28 February 2025), plus the applicable licensing guideline and intended-use classification.

No single one of these standards applies to every humanoid, prosthetic or exoskeleton configuration. Regulatory classification and conformity require product-specific assessment by qualified regulatory and safety professionals.

## 11. Execution status

Authored:
- this physical measurement protocol;
- spatial inertia runtime and replay provenance work in Symtropy PR #4674;
- benchmark and humanoid interaction/resilience requirements.

Not established:
- physical bench construction;
- instrument purchases or calibration;
- actuator selection;
- prototype torque/energy/thermal results;
- fatigue or ingress qualification;
- HIL correlation;
- human-subject or clinical results;
- regulatory conformity or market clearance.
