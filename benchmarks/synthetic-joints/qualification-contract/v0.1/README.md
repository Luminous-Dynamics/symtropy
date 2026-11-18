# Synthetic Joint Benchmark Qualification Contract — v0.1

**Owner:** [SYNTH-JOINT-BENCHMARK-CONTRACT-001 #6140](https://github.com/Luminous-Dynamics/symtropy/issues/6140)  
**Parent:** [SYNTH-JOINT-001 #6037](https://github.com/Luminous-Dynamics/symtropy/issues/6037)  
**Scope:** J1 leg/knee, J2 wrist/ankle and J3 tendon finger/gripper.  
**Evidence class:** benchmark contract and source-bounded example only. This document does not validate physical measurements.

## 1. Design intent

A joint is not qualified by its maximum torque alone. Its usable capability depends on the task, motion envelope, transmission, power source, control loop, workspace, thermal limits, impact response, sensor observability and lifecycle state.

This contract stores each result against its exact candidate/configuration, task envelope and installed-system boundary. It preserves individual metrics and evidence provenance. It intentionally defines no aggregate score or universal leaderboard.

## 2. Result identity

Each result record binds:
- a candidate and joint class (J1/J2/J3);
- exact hardware/software/controller identity where disclosed;
- calibration and service state;
- task pose, motion, payload, wrench, impulse and duty;
- complete installed-system mass/volume boundary and named included/excluded components;
- metric values, units, regimes, denominators, source class, verification state and evidence references;
- uncertainty, calibration and run/failure denominators;
- a claim ceiling.

Unknown values must be represented as null with a reason. Zero is a measurement, not a synonym for unknown. A result whose mass denominator is unavailable cannot manufacture a torque-density value.

The JSON Schema is Draft 2020-12. The example pact-j3-source-record.json demonstrates source-reported model metrics, physical measurements, a one-pose test boundary and explicit unknown installed mass/volume.

## 3. Evidence vocabulary

The source_class and verification_state fields are independent.

### Source class
- manufacturer_claim: catalog/spec-sheet statement.
- vendor_affiliated_measurement: measurement reported by a maker or affiliated lab.
- paper_reported_model: model prediction or simulation result in a paper.
- paper_reported_physical_measurement: measurement reported in a paper, without assuming independent reproduction.
- independent_lab_measurement: measurement by an independent lab.
- internal_physical_measurement: Luminous or project-team physical measurement.
- source_transcription: values copied from a declared source.
- derived_arithmetic: a transparent calculation from known source values.
- analytic_reference: known-answer software result.
- numerical_simulation: simulation output from an identified solver/configuration.

### Verification state
- unreviewed: imported but not checked.
- transcribed: source content and units checked, not independently reproduced.
- arithmetic_checked: derived arithmetic independently recomputed.
- analytically_tested: tested against a separate analytic oracle.
- numerically_crosschecked: compared to an independent numerical implementation.
- physically_measured: physical observation with instrumentation record.
- independently_replicated: reproduced by a genuinely independent group or laboratory.

A paper can report a physical measurement without the measurement being independently replicated. A source transcription can be arithmetically correct while the original claim remains unverified. This is why the axes are separate.

## 4. Mandatory metric boundaries

### Torque / force density

Report continuous rated torque separately from peak torque. Every peak record needs its duration and thermal recovery condition. The mass denominator must identify whether it includes motor, reducer, housing, output bearing, driver, controller, wiring, cooling, mounting and remote actuator/power components.

A component-only density must never be compared to an installed-system density. A vendor-listed maximum cannot be treated as a continuous rating.

### Force-region coverage

For a required task wrench region W_req and achieved feasible set W_feasible:

    coverage = volume(W_req intersection W_feasible) / volume(W_req)

Declare the task frame, geometry/measure, force constraints, contact/friction assumptions and pose set. A directional maximum-force query is a different metric, not a substitute.

### Bandwidth

Separate:
- actuator/drivetrain isolated bandwidth;
- closed-loop joint tracking bandwidth under load;
- task-level useful bandwidth at thermal duty.

A model-predicted 5 Hz drivetrain limit is not a measured closed-loop hand bandwidth.

### Task-space stiffness

Report the full stiffness matrix, eigenvalues, coupling, configuration, hysteresis, modulation time and energy. A scalar maximum hides weak axes and non-diagonal coupling.

### Impact

Record the contact waveform, impulse, duration, impact pose, pre-impact operating torque, link/rotor inertia, stiffness and allowable interface torque. The finite-contact series-elastic bound, zero-contact approximation and normalized QDD ratio criterion remain distinct measures (see #6134).

### Energy

Keep bus/source input, actuator-terminal input, mechanical work, regenerative return, standby and auxiliary power separate. Report the result per accepted task unit and the full supply boundary.

### Model error

Preserve signed residuals and uncertainty by pose, force direction, load, speed, temperature, unit and service state. A global average cannot hide the worst pose or a systematic direction error.

### Lifecycle

Report the entire exposure and failure denominator, including censored runs and failed units. Track tendon/gear/spring wear, creep, backlash, fatigue, calibration drift, replacement, repair time and post-service requalification.

## 5. Source-anchored baselines and stretch targets

These values establish separate starting points—not a ranking:

| Joint | Comparator | Source-reported reference | How to compare |
|---|---|---|---|
| J1 | Huang et al. task-oriented actuator | 0.729 kg, 26 N m rated; 35.7 N m/kg from reported values; guide jump rise 0.840 m at twice-rated command | Reproduce the same two-DoF guide, 5.5 kg load, 48 V and full command sweep; keep simulated G1 demands separate |
| J1 | Unitree GO-M8010-6 | approx. 0.530 kg and 23.7 N m maximum torque; derived 44.7 N m/kg peak/approximate-mass | Peak only, manufacturer spec; do not compare to rated continuous density |
| J2 | Tsai et al. serial wrist | 0.59 kg; 8.10/8.30 N m continuous pitch/yaw; 14 N m/kg | Verify mass numerator/denominator and exact wrist-only/installed boundary; match workspace and stiffness tasks |
| J2 | Hu et al. parallel/origami wrist | 3-PRS/SS; Yoshimura origami; antagonistic tendon/pneumatic stiffness | Quantitative matched mass, continuous torque and installed energy remain unknown until source tables are extracted |
| J2 | Chu et al. experimental parallel wrist (2014) | 7.20 N m/kg reported | Older comparator; duty, mass boundary and workspace not yet matched |
| J3 | PACT finger | Minimum coverage 0.24 baseline → 0.88 frictionless optimized; 0.75 after loss-aware rescoring; 5 Hz modelled isolated-joint F/E minimum | Reproduce the full pose force-region metric; source physical force check is one pose only |
| J3 | Antagonistic tendon hand preprint | 29 N max fingertip force; 13.2 Hz average bandwidth; 220 g/200 mm | Preprint; do not compare maximum force against another design's average force |
| J3 | Tendon/linkage hybrid hand | approx. 4.9 N average fingertip force | Different task/protocol from maximum-force claims; normalize contact, pose and test duration |

The current public actuator register is useful for finding candidate modules, but its November 2026 snapshot has 22 runs across 17 listed actuators and no lab/community submissions. Record whether each result is a maker claim, affiliated measurement or independent test. Register: https://actuatorlist.com/benchmarks.

## 6. Promotion gates

- M0 — traceability: source version, section/table, exact units, numerator, denominator, test setup and claim type.
- M1 — analytic correctness: exact known answers, units, negative controls, fail-closed domain handling and deterministic replay.
- M2 — independent simulation: cross-solver comparison, convergence, event handling, held-out parameter/task strata and uncertainty coverage.
- M3 — instrumented component: calibrated external torque/wrench, motion, current/voltage, thermal and independent truth measurements; raw trace digest, configuration, calibration and repeatability.
- M4 — integrated task: accepted outcomes, payload/pose generalization, interventions, failures and task energy.
- M5 — lifecycle/service: aged and remounted units, degradation, faults, repair, recalibration and requalification.

Never promote a model prediction into physical evidence. Queued CI is not a test pass. A software oracle validates its declared equation/fixture, not the hardware.

## 7. Prospective benchmark targets

- J1: first reproduce the 0.840 m twice-rated guide result and all four source command-sweep observations; then improve a named task/energy/thermal Pareto dimension under identical boundaries.
- J2: reproduce the paper's reported 14 N m/kg using its original denominator, then improve the installed-system frontier while retaining full reachable orientation, cross-axis stiffness control, backdrivability and impact behavior.
- J3: reproduce PACT's complete eight-pose force-region calculation and 5 Hz isolated F/E bandwidth. Stretch: loss-aware minimum coverage >=0.88 at the same pose set and source force-region definition, while retaining the bandwidth and reporting physical uncertainty.
- Impact: qualify against the exact impulse/contact-time/load-inertia envelope. No universal ratio or spring stiffness target is defensible.
- Lifecycle: choose cycle/degradation requirements from an application load spectrum and test repeatability, not a universal cycle count.

These are targets, not achieved Luminous results.

## 8. Non-claims and safety

The contract is a format for evidence and a common comparison protocol. It does not establish the truth of submitted data, safety certification, regulatory conformity, whole-hand grasp closure, human-contact safety or superiority of any architecture. Physical actuation, pressure, impact and human-contact trials require their own requirements, hazard analysis, procedures and independent competent approval.
