# Actuator Thermal Mission Profile v0.1

**Status:** M0/M1 record contract and source transcription. This directory contains no new physical tests or independent measurements.

**Parent work:** [ACTUATOR-THERMAL-MISSION-001 #6155](https://github.com/Luminous-Dynamics/symtropy/issues/6155)  
**Shared result contract:** [#6140](https://github.com/Luminous-Dynamics/symtropy/issues/6140)  
**Benchmark portfolio:** [#6037](https://github.com/Luminous-Dynamics/symtropy/issues/6037)

## Why a thermal mission profile?

A motor's thermal capability is a function of operating history and installation, not simply a continuous-torque scalar. A cold-start hold, a repeated mission with partial cooldown, a locked-rotor test and a dynamic-load trajectory are different experiments. A fair comparison must preserve the distinction.

Three relevant 2026 references:

1. Torres Rodríguez et al., *Toward Reproducible Actuator Characterization: A Comprehensive Framework for Robotics Applications*, **Actuators 15(6), 301**, 30 May 2026. It defines eleven no-load, locked-rotor and dynamic-load protocols and reports metrics such as load robustness and usable bandwidth under load. Its authors position the indicators as descriptive complements to control/system-identification methods, not replacements. DOI: https://doi.org/10.3390/act15060301
2. Kim et al., *Development of a Testbed and Quantitative Evaluation Framework for Characterization of Robot Actuator Dynamics*, ICRA 2026 poster. HYPERDYNE combines no-load, fixed-load and interaction configurations on one reconfigurable testbed and extracts backlash, friction, inertia and frequency response. https://vbn.aau.dk/en/publications/development-of-a-testbed-and-quantitative-evaluation-framework-for-characterization-of-robot-actuator-dynamics/
3. The ActuatorList benchmark register (snapshot updated 16 Nov 2026) reports 22 published runs across 17 catalog actuators, but zero lab and zero community runs in its 3 Nov snapshot. Its thermal entries expose differences in ambient temperature, speed, heatsink dimensions, winding limit and source method. https://actuatorlist.com/benchmarks

The opportunity is to connect these static/dynamic characterization protocols to **repeated robot mission duty** and to publish the evidence in a machine-readable, uncertainty-aware format.

## Paradigm shift: torque-time curve → stateful mission envelope

The conceptual capability function is:

```text
T_out,allow = f(
  output_speed,
  elapsed_time,
  initial_thermal_state,
  ambient_temperature,
  airflow_and_cooling,
  mounting_and_heat_sink,
  preceding_torque_speed_history,
  control_and_current_limits,
  exact_actuator_configuration
)
```

This is deliberately not a presumed one-time-constant thermal model. Identify a model only after the available channels and excitation support it. Winding, stator, case, gearbox and driver temperatures are separate observables; case temperature is not automatically a winding-temperature measurement.

## Schema

- `thermal_mission_profile.schema.json` is a JSON Schema Draft 2020-12 contract.
- `examples/qdd100-beta3-vendor-source-transcription.json` is a source-bounded vendor example, not an independent measurement.

The schema distinguishes:
- locked rotor, fixed load, dynamic load, interaction and mission-cycle trials;
- thermal initial state, reset and cooling/mount boundary;
- reported/estimated/measured temperature channels;
- derating, protection and thermal-limit status;
- exact reported, left-censored, right-censored, not-reached and "continuous" claim time points;
- raw traces from source summaries;
- manufacturer, vendor-affiliated and independent evidence classes.

Unknown values must remain null with an explicit reason. The schema cannot establish the truth of a source claim or validate the quality of a measurement.

## Example interpretation: qdd100 beta 3

ActuatorList reports a beta 3 thermal curve with four points: 16 N·m below 1 s, 10 N·m at 60 s, 6 N·m at 400 s, and 3.3 N·m labelled continuous. Its source note attributes the data to the manufacturer's product page and describes zero-speed operation, 24 V, 20 °C ambient, no airflow and no heat sink. The register assigns ±0.5 N·m as its own estimate; the manufacturer did not state that uncertainty.

The example deliberately preserves:
- the 16 N·m point as **less than 1 s**, not exactly 1 s;
- "continuous" as a vendor label without an invented observation duration;
- unknown initial winding/case temperatures, thermal threshold, sensor location and raw traces;
- the vendor evidence grade rather than lab or independent status.

Do not combine these beta 3 points with the older 2020 qdd100 beta revision. Do not compare the curve directly to dynamic torque-speed or mission performance. The values are a source-transcription example, not a Luminous test result.

## Minimum M0–M5 protocol

### M0 — source and boundary traceability

Bind exact hardware, controller, firmware, configuration, voltage/current limits, ambient temperature, orientation, airflow, heatsink geometry/material, mounting boundary, thermal-limit source and data provenance. For each unknown, record why it is unavailable.

### M1 — record correctness

Validate schema conformance and test negative cases: impossible timestamps, duplicate IDs, contradictory units, unknown temperature encoded as zero, "continuous" with fabricated finite duration, unlabelled left/right censoring, and a vendor source mislabeled independent.

### M2 — numerical model/reconstruction

Where traces exist, reconstruct the torque-time/thermal response independently. Compare candidate thermal model classes on frozen held-out mission profiles and installations. Publish residuals, uncertainty and prediction-interval coverage; do not choose a model merely because it fits one curve.

### M3 — guarded component rig

For any physical campaign, use independently calibrated output torque/speed, bus voltage/current and separately placed winding/stator/case/driver temperature channels. Record actual applied output rather than only commands. Preserve sensor calibration and raw synchronized traces.

### M4 — integrated mission profile

Run a preregistered workload sequence with known initial thermal state, duty cycle, cooldown, payload, speed, direction and environment. Record task success, derating and interruption. A motor-only test does not substitute for installed joint/hand/leg duty.

### M5 — lifecycle and requalification

Repeat with identified aged/remounted/repaired articles, service history and post-service calibration. An installation or cooling-boundary change invalidates a directly transferred thermal qualification unless its equivalence is demonstrated.

## Comparison rules

Never collapse the following:
- zero-speed locked rotor versus dynamic output motion;
- cold start versus hot start or partial recovery;
- motor-only versus driver/heat-sink/mounting installed-system boundary;
- different ambient, airflow, coolant, heatsink or winding-limit conditions;
- measured temperatures versus model-estimated temperatures;
- vendor specifications, vendor tests, digitized plots and independent lab measurements;
- thermal derating time versus actual task completion probability.

Report time to derating onset, time to protection, exposure/censoring, recovery time, energy and thermal gradients independently. Keep every failed, aborted, censored and derated run in the denominator.

## Prospective targets

- Every thermal curve identifies or explicitly marks unknown: test class, speed, ambient, initial state, cooling/mount boundary, thermal limit and evidence class.
- Zero cross-class comparisons that treat locked-rotor, fixed-load and dynamic-load data as interchangeable.
- Repeated mission profiles retain all derating/shutdown/censored outcomes.
- Held-out mission predictions include interval coverage and uncertainty stratified by profile, thermal start, installation and service state.
- Demonstrate improved task-completion energy/time trade-offs without exceeding the declared thermal envelope.

These are protocol targets, not achieved results. Numeric statistical thresholds must be set after instrument repeatability, measurement uncertainty, sample size and application requirements are established.

## Safety and claim ceiling

This contract and source transcription do not authorize energized tests, stall tests, deliberate thermal-limit trips, fabrication or human-contact trials. Physical work requires a separately approved guarded rig, qualified operators, interlocks, manufacturer operating limits, hazard analysis and independent instrumentation. A schema pass, simulation pass or CI pass is not a hardware safety qualification.
